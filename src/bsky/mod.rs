use atrium_api::types::string::{AtIdentifier, Cid, Datetime, Did, Handle, Nsid};
use atrium_api::{
    agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::{
        feed::{defs, get_timeline, post},
        notification,
    },
    com::atproto::{repo, server},
    records,
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use eyre::Result;

pub type Agent = AtpAgent<MemorySessionStore, ReqwestClient>;

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
    // let mut agent = AtpAgent::new(ReqwestClient::new("https://bsky.social".into()));
    let agent = AtpAgent::new(
        ReqwestClient::new("https://bsky.social"),
        MemorySessionStore::default(),
    );
    let session = session(&agent, email, password).await?;
    agent.resume_session(session).await?;
    Ok(agent)
}

pub async fn timeline(agent: &Agent, cursor: Option<String>) -> Result<get_timeline::Output> {
    let timeline = agent
        .api
        .app
        .bsky
        .feed
        .get_timeline(atrium_api::app::bsky::feed::get_timeline::Parameters {
            algorithm: None,
            cursor: cursor.clone(),
            limit: None,
        })
        .await?;

    Ok(timeline)
}

pub async fn send_post(
    agent: &Agent,
    did: Did,
    text: String,
    reply: Option<post::ReplyRef>,
) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
            collection: Nsid::new("app.bsky.feed.post".to_string()).unwrap(),
            record: records::Record::Known(records::KnownRecord::AppBskyFeedPost(Box::new(
                post::Record {
                    created_at: Datetime::now(),
                    embed: None,
                    entities: None,
                    facets: None,
                    langs: None,
                    labels: None,
                    tags: None,
                    reply,
                    text,
                },
            ))),
            repo: AtIdentifier::Did(did),
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
            collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
            repo: AtIdentifier::Did(Did::new(did).unwrap()),
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
            collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
            repo: AtIdentifier::Did(Did::new(did).unwrap()),
            cursor: None,
            limit: None,
            reverse: None,
            rkey_end: None,
            rkey_start: None,
        })
        .await?;

    Ok(reposts)
}

pub async fn toggle_like(agent: &Agent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(like) = viewer.like {
            unlike(agent, did, uri_to_rkey(like).unwrap()).await?;
        } else {
            like(agent, did, feed.post.cid, feed.post.uri).await?;
        }
    }

    Ok(())
}

pub async fn like(agent: &Agent, did: Did, cid: Cid, uri: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
            collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
            record: records::Record::Known(records::KnownRecord::AppBskyFeedLike(Box::new(
                atrium_api::app::bsky::feed::like::Record {
                    created_at: Datetime::now(),
                    subject: repo::strong_ref::Main {
                        cid: cid.clone(),
                        uri: uri.clone(),
                    },
                },
            ))),
            repo: AtIdentifier::Did(did),
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unlike(agent: &Agent, did: Did, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(repo::delete_record::Input {
            collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
            repo: AtIdentifier::Did(did),
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn repost(agent: &Agent, did: Did, cid: Cid, uri: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .create_record(repo::create_record::Input {
            collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
            record: records::Record::Known(records::KnownRecord::AppBskyFeedRepost(Box::new(
                atrium_api::app::bsky::feed::repost::Record {
                    created_at: Datetime::now(),
                    subject: repo::strong_ref::Main {
                        cid: cid.clone(),
                        uri: uri.clone(),
                    },
                },
            ))),
            repo: AtIdentifier::Did(did),
            rkey: None,
            swap_commit: None,
            validate: None,
        })
        .await?;

    Ok(())
}

pub async fn unrepost(agent: &Agent, did: Did, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(repo::delete_record::Input {
            collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
            repo: AtIdentifier::Did(did),
            rkey,
            swap_commit: None,
            swap_record: None,
        })
        .await?;

    Ok(())
}

pub async fn toggle_repost(agent: &Agent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = feed.post.viewer {
        if let Some(repost) = viewer.repost {
            unrepost(agent, did, uri_to_rkey(repost).unwrap()).await?;
        } else {
            repost(agent, did, feed.post.cid, feed.post.uri).await?;
        }
    }

    Ok(())
}

pub fn get_url(handle: Handle, uri: String) -> Option<String> {
    if let Some(id) = uri.split('/').last() {
        let url = format!(
            "https://bsky.app/profile/{}/post/{}",
            handle.to_string(),
            id
        );
        Some(url.clone())
    } else {
        None
    }
}

pub fn uri_to_rkey(uri: String) -> Option<String> {
    uri.split('/').last().map(|s| s.to_string())
}
