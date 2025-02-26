use atrium_api::record::KnownRecord;
use atrium_api::types::string::{AtIdentifier, Cid, Datetime, Did, Handle, Nsid};
use atrium_api::{
    agent::atp_agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::{
        feed::{defs, get_timeline, post, search_posts},
        notification,
    },
    com::atproto::{repo, server},
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use bsky_sdk::BskyAgent;
use eyre::Result;

pub type Agent = AtpAgent<MemorySessionStore, ReqwestClient>;

pub async fn session(
    agent: &BskyAgent,
    email: String,
    password: String,
) -> Result<server::create_session::Output> {
    let session = agent.login(email, password).await?;
    Ok(session)
}

pub async fn agent_with_session(email: String, password: String) -> Result<BskyAgent> {
    let agent = BskyAgent::builder().build().await?;
    let session = agent.login(email, password).await?;
    agent.resume_session(session).await?;
    Ok(agent)
}

pub async fn timeline(agent: &BskyAgent, cursor: Option<String>) -> Result<get_timeline::Output> {
    let timeline = agent
        .api
        .app
        .bsky
        .feed
        .get_timeline(
            get_timeline::ParametersData {
                algorithm: None,
                cursor: cursor.clone(),
                limit: None,
            }
            .into(),
        )
        .await?;

    Ok(timeline)
}

pub async fn search(agent: &BskyAgent, query: String) -> Result<search_posts::Output> {
    let search_result = agent
        .api
        .app
        .bsky
        .feed
        .search_posts(
            search_posts::ParametersData {
                cursor: None,
                limit: None,
                q: query.clone(),
                author: None,
                domain: None,
                lang: None,
                mentions: None,
                since: None,
                sort: None,
                tag: None,
                until: None,
                url: None,
            }
            .into(),
        )
        .await?;

    Ok(search_result)
}

pub async fn send_post(
    agent: &BskyAgent,
    _did: Did,
    text: String,
    reply: Option<post::ReplyRef>,
) -> Result<()> {
    agent
        .create_record(post::RecordData {
            created_at: Datetime::now(),
            embed: None,
            entities: None,
            facets: None,
            langs: None,
            labels: None,
            tags: None,
            reply,
            text,
        })
        .await?;

    Ok(())
}

pub async fn notifications(agent: &BskyAgent) -> Result<notification::list_notifications::Output> {
    let notifications = agent
        .api
        .app
        .bsky
        .notification
        .list_notifications(
            notification::list_notifications::ParametersData {
                cursor: None,
                limit: None,
                priority: None,
                reasons: None,
                seen_at: None,
            }
            .into(),
        )
        .await?;

    Ok(notifications)
}

pub async fn likes(agent: &BskyAgent, did: String) -> Result<repo::list_records::Output> {
    let likes = agent
        .api
        .com
        .atproto
        .repo
        .list_records(
            repo::list_records::ParametersData {
                collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
                repo: AtIdentifier::Did(Did::new(did).unwrap()),
                cursor: None,
                limit: None,
                reverse: None,
                rkey_end: None,
                rkey_start: None,
            }
            .into(),
        )
        .await?;

    Ok(likes)
}

pub async fn reposts(agent: &BskyAgent, did: String) -> Result<repo::list_records::Output> {
    let reposts = agent
        .api
        .com
        .atproto
        .repo
        .list_records(
            repo::list_records::ParametersData {
                collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
                repo: AtIdentifier::Did(Did::new(did).unwrap()),
                cursor: None,
                limit: None,
                reverse: None,
                rkey_end: None,
                rkey_start: None,
            }
            .into(),
        )
        .await?;

    Ok(reposts)
}

pub async fn toggle_like(agent: &BskyAgent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = &feed.post.viewer {
        if let Some(like) = &viewer.like {
            unlike(agent, did, uri_to_rkey(like.clone()).unwrap()).await?;
        } else {
            like(agent, did, feed.post.cid.clone(), feed.post.uri.clone()).await?;
        }
    }

    Ok(())
}

pub async fn like(agent: &BskyAgent, _did: Did, cid: Cid, uri: String) -> Result<()> {
    agent
        .create_record(KnownRecord::AppBskyFeedLike(Box::new(
            atrium_api::app::bsky::feed::like::RecordData {
                created_at: Datetime::now(),
                subject: repo::strong_ref::MainData {
                    cid: cid.clone(),
                    uri: uri.clone(),
                }
                .into(),
            }
            .into(),
        )))
        .await?;

    Ok(())
}

pub async fn unlike(agent: &Agent, did: Did, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(
            repo::delete_record::InputData {
                collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
                repo: AtIdentifier::Did(did),
                rkey,
                swap_commit: None,
                swap_record: None,
            }
            .into(),
        )
        .await?;

    Ok(())
}

pub async fn repost(agent: &BskyAgent, _did: Did, cid: Cid, uri: String) -> Result<()> {
    agent
        .create_record(KnownRecord::AppBskyFeedRepost(Box::new(
            atrium_api::app::bsky::feed::repost::RecordData {
                created_at: Datetime::now(),
                subject: repo::strong_ref::MainData {
                    cid: cid.clone(),
                    uri: uri.clone(),
                }
                .into(),
            }
            .into(),
        )))
        .await?;

    Ok(())
}

pub async fn unrepost(agent: &BskyAgent, did: Did, rkey: String) -> Result<()> {
    agent
        .api
        .com
        .atproto
        .repo
        .delete_record(
            repo::delete_record::InputData {
                collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
                repo: AtIdentifier::Did(did),
                rkey,
                swap_commit: None,
                swap_record: None,
            }
            .into(),
        )
        .await?;

    Ok(())
}

pub async fn toggle_repost(agent: &BskyAgent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(viewer) = &feed.post.viewer {
        if let Some(repost) = &viewer.repost {
            unrepost(agent, did, uri_to_rkey(repost.clone()).unwrap()).await?;
        } else {
            repost(agent, did, feed.post.cid.clone(), feed.post.uri.clone()).await?;
        }
    }

    Ok(())
}

pub fn get_url(handle: Handle, uri: String) -> Option<String> {
    if let Some(id) = uri.split('/').last() {
        let handle = handle.to_string();
        Some(format!("https://bsky.app/profile/{handle}/post/{id}"))
    } else {
        None
    }
}

pub fn uri_to_rkey(uri: String) -> Option<String> {
    uri.split('/').last().map(|s| s.to_string())
}
