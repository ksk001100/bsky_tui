//! reactions Bluesky services.

use super::*;

pub async fn likes(agent: &BskyAgent, did: String) -> Result<repo::list_records::Output> {
    let likes = agent
        .api
        .com
        .atproto
        .repo
        .list_records(
            repo::list_records::ParametersData {
                collection: Nsid::new("app.bsky.feed.like".to_string())
                    .map_err(eyre::Report::msg)?,
                repo: AtIdentifier::Did(Did::new(did).map_err(eyre::Report::msg)?),
                cursor: None,
                limit: None,
                reverse: None,
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
                collection: Nsid::new("app.bsky.feed.repost".to_string())
                    .map_err(eyre::Report::msg)?,
                repo: AtIdentifier::Did(Did::new(did).map_err(eyre::Report::msg)?),
                cursor: None,
                limit: None,
                reverse: None,
            }
            .into(),
        )
        .await?;

    Ok(reposts)
}

pub async fn toggle_like(agent: &BskyAgent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(like_uri) = feed
        .post
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.like.as_ref())
    {
        let rkey = uri_to_rkey(like_uri.clone()).ok_or_else(|| eyre::eyre!("invalid Like URI"))?;
        unlike(agent, did, rkey).await?;
    } else {
        like(agent, did, feed.post.cid.clone(), feed.post.uri.clone()).await?;
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
                via: None,
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
                collection: Nsid::new("app.bsky.feed.like".to_string())
                    .map_err(eyre::Report::msg)?,
                repo: AtIdentifier::Did(did),
                rkey: RecordKey::new(rkey).map_err(eyre::Report::msg)?,
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
                via: None,
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
                collection: Nsid::new("app.bsky.feed.repost".to_string())
                    .map_err(eyre::Report::msg)?,
                repo: AtIdentifier::Did(did),
                rkey: RecordKey::new(rkey).map_err(eyre::Report::msg)?,
                swap_commit: None,
                swap_record: None,
            }
            .into(),
        )
        .await?;

    Ok(())
}

pub async fn toggle_repost(agent: &BskyAgent, did: Did, feed: defs::FeedViewPost) -> Result<()> {
    if let Some(repost_uri) = feed
        .post
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.repost.as_ref())
    {
        let rkey =
            uri_to_rkey(repost_uri.clone()).ok_or_else(|| eyre::eyre!("invalid Repost URI"))?;
        unrepost(agent, did, rkey).await?;
    } else {
        repost(agent, did, feed.post.cid.clone(), feed.post.uri.clone()).await?;
    }

    Ok(())
}

pub fn get_url(handle: Handle, uri: String) -> Option<String> {
    if let Some(id) = uri.split('/').next_back() {
        let handle = handle.to_string();
        Some(format!("https://bsky.app/profile/{handle}/post/{id}"))
    } else {
        None
    }
}

pub fn notification_post_url(
    notification: &notification::list_notifications::Notification,
    own_handle: &Handle,
) -> Option<String> {
    match notification.reason.as_str() {
        "reply" | "mention" | "quote" => {
            get_url(notification.author.handle.clone(), notification.uri.clone())
        }
        "like" | "repost" => notification
            .reason_subject
            .as_ref()
            .and_then(|uri| get_url(own_handle.clone(), uri.clone())),
        _ => None,
    }
}

pub fn uri_to_rkey(uri: String) -> Option<String> {
    uri.split('/').next_back().map(|s| s.to_string())
}

pub async fn toggle_like_post_view(
    agent: &BskyAgent,
    did: Did,
    post: defs::PostViewData,
) -> Result<()> {
    if let Some(like_uri) = post.viewer.as_ref().and_then(|viewer| viewer.like.as_ref()) {
        let rkey = uri_to_rkey(like_uri.clone()).ok_or_else(|| eyre::eyre!("invalid Like URI"))?;
        unlike(agent, did, rkey).await?;
    } else {
        like(agent, did, post.cid.clone(), post.uri.clone()).await?;
    }

    Ok(())
}

pub async fn toggle_repost_post_view(
    agent: &BskyAgent,
    did: Did,
    post: defs::PostViewData,
) -> Result<()> {
    if let Some(repost_uri) = post
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.repost.as_ref())
    {
        let rkey =
            uri_to_rkey(repost_uri.clone()).ok_or_else(|| eyre::eyre!("invalid Repost URI"))?;
        unrepost(agent, did, rkey).await?;
    } else {
        repost(agent, did, post.cid.clone(), post.uri.clone()).await?;
    }

    Ok(())
}
