use atrium_api::{
    agent::{AtpAgent, BaseClient},
    app::bsky::{
        feed::{defs, get_timeline, post},
        notification,
    },
    com::atproto::{repo, server},
    records,
};
use atrium_xrpc::client::reqwest::ReqwestClient;
use chrono::Utc;
use eyre::Result;

pub type Agent = AtpAgent<BaseClient<ReqwestClient>>;

pub async fn session(
    agent: &Agent,
    email: String,
    password: String,
) -> Result<server::create_session::Output> {
    let session = agent
        .api
        .com
        .atproto
        .server
        .create_session(server::create_session::Input {
            // TODO: use env vars
            identifier: email,
            password,
        })
        .await?;

    Ok(session)
}

pub async fn agent_with_session(email: String, password: String) -> Result<Agent> {
    let mut agent = AtpAgent::new(ReqwestClient::new("https://bsky.social".into()));
    let session = session(&agent, email, password).await?;
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
pub async fn send_post(
    agent: &Agent,
    did: String,
    text: String,
    reply: Option<post::ReplyRef>,
) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
            collection: String::from("app.bsky.feed.post"),
            record: records::Record::AppBskyFeedPost(Box::new(post::Record {
                created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                embed: None,
                entities: None,
                facets: None,
                langs: None,
                reply,
                text,
            })),
            repo: did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn notifications(agent: &Agent) -> Result<notification::list_notifications::Output> {
    let notifications = agent
        .api
        .app
        .bsky
        .notification
        .list_notifications(notification::list_notifications::Parameters {
            cursor: None,
            limit: None,
            seen_at: None,
        })
        .await?;

    Ok(notifications)
}

pub async fn likes(agent: &Agent, did: String) -> Result<repo::list_records::Output> {
    let likes = agent
        .api
        .com
        .atproto
        .repo
        .list_records(repo::list_records::Parameters {
            collection: String::from("app.bsky.feed.like"),
            repo: did,
            cursor: None,
            limit: None,
            reverse: None,
            rkey_end: None,
            rkey_start: None,
        })
        .await?;

    Ok(likes)
}

pub async fn reposts(agent: &Agent, did: String) -> Result<repo::list_records::Output> {
    let reposts = agent
        .api
        .com
        .atproto
        .repo
        .list_records(repo::list_records::Parameters {
            collection: String::from("app.bsky.feed.repost"),
            repo: did,
            cursor: None,
            limit: None,
            reverse: None,
            rkey_end: None,
            rkey_start: None,
        })
        .await?;

    Ok(reposts)
}

pub async fn toggle_like(agent: &Agent, did: String, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(like) = viewer.like {
            unlike(agent, did, uri_to_rkey(like).unwrap()).await?;
        } else {
            like(agent, did, feed.post.cid, feed.post.uri).await?;
        }
    }

    Ok(())
}

pub async fn like(agent: &Agent, did: String, cid: String, uri: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
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
            repo: did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unlike(agent: &Agent, did: String, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(repo::delete_record::Input {
            collection: String::from("app.bsky.feed.like"),
            repo: did,
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn repost(agent: &Agent, did: String, cid: String, uri: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
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
            repo: did,
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unrepost(agent: &Agent, did: String, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(repo::delete_record::Input {
            collection: String::from("app.bsky.feed.repost"),
            repo: did,
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn toggle_repost(agent: &Agent, did: String, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(repost) = viewer.repost {
            unrepost(agent, did, uri_to_rkey(repost).unwrap()).await?;
        } else {
            repost(agent, did, feed.post.cid, feed.post.uri).await?;
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
    uri.split('/').last().map(|s| s.to_string())
}
