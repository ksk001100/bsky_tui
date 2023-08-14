use atrium_api::agent::{AtpAgent, BaseClient};
use atrium_api::app::bsky::feed::defs::FeedViewPost;
use atrium_api::app::bsky::feed::get_timeline;
use atrium_api::app::bsky::feed::post;
use atrium_api::app::bsky::notification::list_notifications;
use atrium_api::com::atproto::repo::{self, create_record, delete_record, list_records};
use atrium_api::com::atproto::server::create_session;
use atrium_api::records;
use atrium_xrpc::client::reqwest::ReqwestClient;
use chrono::Utc;
use eyre::Result;

pub type Agent = AtpAgent<BaseClient<ReqwestClient>>;

pub async fn session(agent: &Agent) -> Result<create_session::Output> {
    let session = agent
        .api
        .com
        .atproto
        .server
        .create_session(create_session::Input {
            // TODO: use env vars
            identifier: env!("BLUESKY_EMAIL").into(),
            password: env!("BLUESKY_PASSWORD").into(),
        })
        .await?;

    Ok(session)
}

pub async fn agent_with_session() -> Result<Agent> {
    let mut agent = AtpAgent::new(ReqwestClient::new("https://bsky.social".into()));
    let session = session(&agent).await?;
    agent.set_session(session);
    Ok(agent)
}

pub async fn timeline(agent: &Agent) -> Result<get_timeline::Output> {
    let timeline = agent
        .api
        .app
        .bsky
        .feed
        .get_timeline(atrium_api::app::bsky::feed::get_timeline::Parameters {
            algorithm: None,
            cursor: None,
            limit: None,
        })
        .await?;

    Ok(timeline)
}
pub async fn send_post(agent: &Agent, text: String) -> Result<()> {
    let session = session(agent).await?;
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(create_record::Input {
            collection: String::from("app.bsky.feed.post"),
            record: records::Record::AppBskyFeedPost(Box::new(post::Record {
                created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                embed: None,
                entities: None,
                facets: None,
                langs: None,
                reply: None,
                text,
            })),
            repo: session.did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn notifications(agent: &Agent) -> Result<list_notifications::Output> {
    let notifications = agent
        .api
        .app
        .bsky
        .notification
        .list_notifications(list_notifications::Parameters {
            cursor: None,
            limit: None,
            seen_at: None,
        })
        .await?;

    Ok(notifications)
}

pub async fn likes(agent: &Agent) -> Result<list_records::Output> {
    let session = session(agent).await?;
    let likes = agent
        .api
        .com
        .atproto
        .repo
        .list_records(list_records::Parameters {
            collection: String::from("app.bsky.feed.like"),
            repo: session.did,
            cursor: None,
            limit: None,
            reverse: None,
            rkey_end: None,
            rkey_start: None,
        })
        .await?;

    Ok(likes)
}

pub async fn reposts(agent: &Agent) -> Result<list_records::Output> {
    let session = session(agent).await?;
    let reposts = agent
        .api
        .com
        .atproto
        .repo
        .list_records(list_records::Parameters {
            collection: String::from("app.bsky.feed.repost"),
            repo: session.did,
            cursor: None,
            limit: None,
            reverse: None,
            rkey_end: None,
            rkey_start: None,
        })
        .await?;

    Ok(reposts)
}

pub async fn toggle_like(agent: &Agent, feed: FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(like) = viewer.like {
            unlike(agent, uri_to_rkey(like).unwrap()).await?;
        } else {
            like(agent, feed.post.cid, feed.post.uri).await?;
        }
    }

    Ok(())
}

pub async fn like(agent: &Agent, cid: String, uri: String) -> Result<()> {
    let session = session(agent).await?;
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(create_record::Input {
            collection: String::from("app.bsky.feed.like"),
            record: records::Record::AppBskyFeedLike(Box::new(
                atrium_api::app::bsky::feed::like::Record {
                    created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    subject: repo::strong_ref::Main {
                        cid: cid.clone(),
                        uri: uri.clone(),
                    },
                },
            )),
            repo: session.did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unlike(agent: &Agent, rkey: String) -> Result<()> {
    let session = session(agent).await?;
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(delete_record::Input {
            collection: String::from("app.bsky.feed.like"),
            repo: session.did,
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn repost(agent: &Agent, cid: String, uri: String) -> Result<()> {
    let session = session(agent).await?;
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(create_record::Input {
            collection: String::from("app.bsky.feed.repost"),
            record: records::Record::AppBskyFeedRepost(Box::new(
                atrium_api::app::bsky::feed::repost::Record {
                    created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    subject: repo::strong_ref::Main {
                        cid: cid.clone(),
                        uri: uri.clone(),
                    },
                },
            )),
            repo: session.did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unrepost(agent: &Agent, rkey: String) -> Result<()> {
    let session = session(agent).await?;
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(delete_record::Input {
            collection: String::from("app.bsky.feed.repost"),
            repo: session.did,
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn toggle_repost(agent: &Agent, feed: FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(repost) = viewer.repost {
            unrepost(agent, uri_to_rkey(repost).unwrap()).await?;
        } else {
            repost(agent, feed.post.cid, feed.post.uri).await?;
        }
    }

    Ok(())
}

pub fn get_url(handle: String, uri: String) -> Option<String> {
    if let Some(id) = uri.split('/').last() {
        let url = format!("https://bsky.app/profile/{}/post/{}", handle, id);
        Some(url.clone())
    } else {
        None
    }
}

pub fn uri_to_rkey(uri: String) -> Option<String> {
    if let Some(rkey) = uri.split('/').last() {
        Some(rkey.to_string())
    } else {
        None
    }
}
