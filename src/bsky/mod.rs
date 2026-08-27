use std::convert::TryInto;

use eyre::{bail, Result};
use unicode_segmentation::UnicodeSegmentation;

use atrium_api::{
    agent::atp_agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::{
        actor::get_preferences,
        embed::{record::ViewRecordRefs, record_with_media::ViewMediaRefs},
        feed::{defs, get_post_thread, get_timeline, post, search_posts},
        notification,
    },
    com::atproto::{repo, server},
    record::KnownRecord,
    types::{
        string::{AtIdentifier, Cid, Datetime, Did, Handle, Nsid, RecordKey},
        Union,
    },
};
use atrium_xrpc_client::reqwest::ReqwestClient;

use bsky_sdk::api::types::TryFromUnknown;
use bsky_sdk::BskyAgent;

use crate::app::moderation::ModerationPrefs;

pub type Agent = AtpAgent<MemorySessionStore, ReqwestClient>;

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

fn root_thread_post(thread: &defs::ThreadViewPost) -> &defs::PostView {
    match thread.parent.as_ref() {
        Some(Union::Refs(defs::ThreadViewPostParentRefs::ThreadViewPost(parent))) => {
            root_thread_post(parent)
        }
        _ => &thread.post,
    }
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

pub async fn post_thread(agent: &BskyAgent, uri: String) -> Result<get_post_thread::Output> {
    Ok(agent
        .api
        .app
        .bsky
        .feed
        .get_post_thread(
            get_post_thread::ParametersData {
                depth: Some(100_u16.try_into().map_err(eyre::Report::msg)?),
                parent_height: Some(100_u16.try_into().map_err(eyre::Report::msg)?),
                uri,
            }
            .into(),
        )
        .await?)
}

pub async fn search(
    agent: &BskyAgent,
    query: String,
    cursor: Option<String>,
) -> Result<search_posts::Output> {
    let search_result = agent
        .api
        .app
        .bsky
        .feed
        .search_posts(
            search_posts::ParametersData {
                cursor,
                limit: None,
                q: query.clone(),
                author: None,
                domain: None,
                lang: None,
                mentions: None,
                since: None,
                sort: Some("latest".to_string()),
                tag: None,
                until: None,
                url: None,
            }
            .into(),
        )
        .await?;

    Ok(search_result)
}

pub fn post_image_urls(post: &defs::PostViewData, moderation: &ModerationPrefs) -> Vec<String> {
    if !moderation.decision(post).permits_media() {
        return Vec::new();
    }
    let mut urls = Vec::new();
    if let Some(avatar) = &post.author.avatar {
        urls.push(avatar.clone());
    }
    urls.extend(post_attachment_urls(post, moderation));
    urls
}

pub fn post_attachment_urls(
    post: &defs::PostViewData,
    moderation: &ModerationPrefs,
) -> Vec<String> {
    if !moderation.decision(post).permits_media() {
        return Vec::new();
    }
    let mut urls = post_attachments(post)
        .iter()
        .map(|image| image.thumb.clone())
        .collect::<Vec<_>>();
    if let Some(thumbnail) = post_embed_thumbnail(post) {
        urls.push(thumbnail);
    }
    urls
}

pub fn post_attachment_fullsize_urls(
    post: &defs::PostViewData,
    moderation: &ModerationPrefs,
) -> Vec<String> {
    if !moderation.decision(post).permits_media() {
        return Vec::new();
    }
    post_attachments(post)
        .iter()
        .map(|image| image.fullsize.clone())
        .collect()
}

pub fn post_attachment_alt_texts(post: &defs::PostViewData) -> Vec<String> {
    post_attachments(post)
        .iter()
        .map(|image| image.alt.clone())
        .collect()
}

pub fn post_embed_lines(post: &defs::PostViewData) -> Vec<String> {
    let Some(Union::Refs(embed)) = &post.embed else {
        return Vec::new();
    };
    match embed {
        defs::PostViewEmbedRefs::AppBskyEmbedImagesView(_) => Vec::new(),
        defs::PostViewEmbedRefs::AppBskyEmbedVideoView(video) => video_lines(video),
        defs::PostViewEmbedRefs::AppBskyEmbedExternalView(external) => vec![
            format!("Link: {}", external.external.title),
            external.external.description.clone(),
            external.external.uri.clone(),
        ],
        defs::PostViewEmbedRefs::AppBskyEmbedRecordView(record) => record_lines(record),
        defs::PostViewEmbedRefs::AppBskyEmbedRecordWithMediaView(embed) => {
            let mut lines = match &embed.media {
                Union::Refs(ViewMediaRefs::AppBskyEmbedImagesView(_)) => Vec::new(),
                Union::Refs(ViewMediaRefs::AppBskyEmbedVideoView(video)) => video_lines(video),
                Union::Refs(ViewMediaRefs::AppBskyEmbedExternalView(external)) => vec![
                    format!("Link: {}", external.external.title),
                    external.external.description.clone(),
                    external.external.uri.clone(),
                ],
                Union::Unknown(_) => vec!["[Unsupported media embed]".to_owned()],
            };
            lines.extend(record_lines(&embed.record));
            lines
        }
    }
}

pub fn post_embed_url(post: &defs::PostViewData) -> Option<String> {
    let Union::Refs(embed) = post.embed.as_ref()? else {
        return None;
    };
    match embed {
        defs::PostViewEmbedRefs::AppBskyEmbedVideoView(video) => Some(video.playlist.clone()),
        defs::PostViewEmbedRefs::AppBskyEmbedExternalView(external) => {
            Some(external.external.uri.clone())
        }
        defs::PostViewEmbedRefs::AppBskyEmbedRecordWithMediaView(embed) => match &embed.media {
            Union::Refs(ViewMediaRefs::AppBskyEmbedVideoView(video)) => {
                Some(video.playlist.clone())
            }
            Union::Refs(ViewMediaRefs::AppBskyEmbedExternalView(external)) => {
                Some(external.external.uri.clone())
            }
            _ => None,
        },
        _ => None,
    }
}

fn video_lines(video: &atrium_api::app::bsky::embed::video::View) -> Vec<String> {
    vec![
        "Video/GIF (press e to play externally)".to_owned(),
        format!("Alt: {}", video.alt.as_deref().unwrap_or("(not provided)")),
        "Captions: available through the video playlist when provided".to_owned(),
    ]
}

fn record_lines(record: &atrium_api::app::bsky::embed::record::View) -> Vec<String> {
    match &record.record {
        Union::Refs(ViewRecordRefs::ViewRecord(quoted)) => {
            let text = post::Record::try_from_unknown(quoted.value.clone())
                .map(|record| record.text.clone())
                .unwrap_or_else(|_| "[Quoted record unavailable]".to_owned());
            vec![
                format!(
                    "Quote: {} @{}",
                    quoted.author.display_name.clone().unwrap_or_default(),
                    quoted.author.handle.as_str()
                ),
                text,
            ]
        }
        Union::Refs(ViewRecordRefs::ViewNotFound(_)) => {
            vec!["[Quoted post not found or deleted]".to_owned()]
        }
        Union::Refs(ViewRecordRefs::ViewBlocked(_)) => {
            vec!["[Quoted post hidden: blocked author]".to_owned()]
        }
        Union::Refs(ViewRecordRefs::ViewDetached(_)) => {
            vec!["[Quote detached by post author]".to_owned()]
        }
        Union::Refs(_) => vec!["[Quoted non-post record]".to_owned()],
        Union::Unknown(_) => vec!["[Unsupported quoted record]".to_owned()],
    }
}

fn post_attachments(
    post: &defs::PostViewData,
) -> &[atrium_api::app::bsky::embed::images::ViewImage] {
    let Some(Union::Refs(embed)) = &post.embed else {
        return &[];
    };

    match embed {
        defs::PostViewEmbedRefs::AppBskyEmbedImagesView(images) => &images.images,
        defs::PostViewEmbedRefs::AppBskyEmbedRecordWithMediaView(record) => match &record.media {
            Union::Refs(ViewMediaRefs::AppBskyEmbedImagesView(images)) => &images.images,
            _ => &[],
        },
        _ => &[],
    }
}

fn post_embed_thumbnail(post: &defs::PostViewData) -> Option<String> {
    let Union::Refs(embed) = post.embed.as_ref()? else {
        return None;
    };
    match embed {
        defs::PostViewEmbedRefs::AppBskyEmbedVideoView(video) => video.thumbnail.clone(),
        defs::PostViewEmbedRefs::AppBskyEmbedExternalView(external) => {
            external.external.thumb.clone()
        }
        defs::PostViewEmbedRefs::AppBskyEmbedRecordWithMediaView(embed) => match &embed.media {
            Union::Refs(ViewMediaRefs::AppBskyEmbedVideoView(video)) => video.thumbnail.clone(),
            Union::Refs(ViewMediaRefs::AppBskyEmbedExternalView(external)) => {
                external.external.thumb.clone()
            }
            _ => None,
        },
        _ => None,
    }
}

pub async fn send_post(
    agent: &BskyAgent,
    _did: Did,
    text: String,
    reply: Option<post::ReplyRef>,
) -> Result<()> {
    let rich_text = bsky_sdk::rich_text::RichText::new_with_detect_facets(&text).await?;
    agent
        .create_record(post::RecordData {
            created_at: Datetime::now(),
            embed: None,
            entities: None,
            facets: rich_text.facets,
            langs: None,
            labels: None,
            tags: None,
            reply,
            text,
        })
        .await?;

    Ok(())
}

pub async fn notifications(
    agent: &BskyAgent,
    cursor: Option<String>,
) -> Result<notification::list_notifications::Output> {
    let notifications = agent
        .api
        .app
        .bsky
        .notification
        .list_notifications(
            notification::list_notifications::ParametersData {
                cursor,
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

pub async fn update_seen(agent: &BskyAgent) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .update_seen(
            notification::update_seen::InputData {
                seen_at: Datetime::now(),
            }
            .into(),
        )
        .await?;
    Ok(())
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
                collection: Nsid::new("app.bsky.feed.like".to_string()).unwrap(),
                repo: AtIdentifier::Did(did),
                rkey: RecordKey::new(rkey).unwrap(),
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
                collection: Nsid::new("app.bsky.feed.repost".to_string()).unwrap(),
                repo: AtIdentifier::Did(did),
                rkey: RecordKey::new(rkey).unwrap(),
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
