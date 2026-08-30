//! session Bluesky services.

use super::*;

pub async fn agent_with_session(
    service_url: &str,
    identifier: &str,
    password: &str,
) -> Result<(BskyAgent, server::create_session::Output)> {
    let config = bsky_sdk::agent::config::Config {
        endpoint: service_url.to_owned(),
        ..Default::default()
    };
    let agent = BskyAgent::builder().config(config).build().await?;
    let session = agent.login(identifier, password).await?;
    Ok((agent, session))
}

pub async fn moderation_preferences(agent: &BskyAgent) -> Result<ModerationPrefs> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .get_preferences(get_preferences::ParametersData {}.into())
        .await?;
    Ok(ModerationPrefs::from_api(&output.preferences))
}

pub fn validate_post_text(text: &str) -> Result<()> {
    if text.trim().is_empty() {
        bail!("post text cannot be empty");
    }
    let graphemes = text.graphemes(true).count();
    if graphemes > 300 {
        bail!("post is {graphemes} graphemes; the limit is 300");
    }
    if text.len() > 3000 {
        bail!("post is {} bytes; the limit is 3000", text.len());
    }
    Ok(())
}

pub fn reply_root(feed: &defs::FeedViewPost) -> Option<repo::strong_ref::MainData> {
    let reply = feed.reply.as_ref()?;
    let Union::Refs(defs::ReplyRefRootRefs::PostView(root)) = &reply.root else {
        return None;
    };
    Some(repo::strong_ref::MainData {
        cid: root.cid.clone(),
        uri: root.uri.clone(),
    })
}

pub async fn reply_root_for_post(
    agent: &BskyAgent,
    post: &defs::PostViewData,
) -> Result<repo::strong_ref::MainData> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_post_thread(
            get_post_thread::ParametersData {
                depth: Some(0_u16.try_into().map_err(eyre::Report::msg)?),
                parent_height: Some(1000_u16.try_into().map_err(eyre::Report::msg)?),
                uri: post.uri.clone(),
            }
            .into(),
        )
        .await?;
    let Union::Refs(get_post_thread::OutputThreadRefs::AppBskyFeedDefsThreadViewPost(thread)) =
        &output.thread
    else {
        bail!("the reply target is unavailable");
    };
    let root = root_thread_post(thread);
    Ok(repo::strong_ref::MainData {
        cid: root.cid.clone(),
        uri: root.uri.clone(),
    })
}

pub(super) fn root_thread_post(thread: &defs::ThreadViewPost) -> &defs::PostView {
    match thread.parent.as_ref() {
        Some(Union::Refs(defs::ThreadViewPostParentRefs::ThreadViewPost(parent))) => {
            root_thread_post(parent)
        }
        _ => &thread.post,
    }
}
