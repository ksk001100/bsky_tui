use std::convert::TryInto;

use eyre::{bail, Result};
use unicode_segmentation::UnicodeSegmentation;

use atrium_api::{
    agent::atp_agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::{
        actor::{get_preferences, get_profile, search_actors},
        embed::{record::ViewRecordRefs, record_with_media::ViewMediaRefs},
        feed::{
            defs, get_actor_feeds, get_actor_likes, get_author_feed, get_likes, get_post_thread,
            get_quotes, get_reposted_by, get_timeline, post, search_posts,
        },
        graph::{
            follow, get_actor_starter_packs, get_followers, get_follows, get_lists, starterpack,
        },
        notification,
        richtext::facet,
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
use crate::app::profile::{ProfileContent, ProfileListItem, ProfileSection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostFacet {
    pub label: String,
    pub kind: &'static str,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractionItem {
    pub title: String,
    pub subtitle: String,
    pub url: String,
    pub actor: Option<AtIdentifier>,
}

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

pub async fn post_likes(agent: &BskyAgent, uri: String, cid: Cid) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_likes(
            get_likes::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .likes
        .iter()
        .map(|like| profile_interaction(&like.actor))
        .collect())
}

pub async fn post_reposts(
    agent: &BskyAgent,
    uri: String,
    cid: Cid,
) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_reposted_by(
            get_reposted_by::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .reposted_by
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn post_quotes(agent: &BskyAgent, uri: String, cid: Cid) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .feed
        .get_quotes(
            get_quotes::ParametersData {
                cid: Some(cid),
                cursor: None,
                limit: None,
                uri,
            }
            .into(),
        )
        .await?;
    Ok(output
        .posts
        .iter()
        .map(|post| InteractionItem {
            title: format!(
                "{} @{}",
                post.author.display_name.clone().unwrap_or_default(),
                post.author.handle.as_str()
            ),
            subtitle: post::Record::try_from_unknown(post.record.clone())
                .map(|record| record.text.clone())
                .unwrap_or_else(|_| "[Post unavailable]".to_owned()),
            url: get_url(post.author.handle.clone(), post.uri.clone()).unwrap_or_default(),
            actor: None,
        })
        .collect())
}

fn profile_interaction(
    profile: &atrium_api::app::bsky::actor::defs::ProfileViewData,
) -> InteractionItem {
    InteractionItem {
        title: profile.display_name.clone().unwrap_or_default(),
        subtitle: format!("@{}", profile.handle.as_str()),
        url: format!("https://bsky.app/profile/{}", profile.did.as_str()),
        actor: Some(profile.did.clone().into()),
    }
}

pub async fn profile(
    agent: &BskyAgent,
    actor: AtIdentifier,
) -> Result<atrium_api::app::bsky::actor::defs::ProfileViewDetailed> {
    Ok(agent
        .api
        .app
        .bsky
        .actor
        .get_profile(get_profile::ParametersData { actor }.into())
        .await?)
}

pub async fn profile_content(
    agent: &BskyAgent,
    actor: AtIdentifier,
    section: ProfileSection,
) -> Result<ProfileContent> {
    match section {
        ProfileSection::Posts | ProfileSection::Replies | ProfileSection::Media => {
            let filter = match section {
                ProfileSection::Posts => "posts_no_replies",
                ProfileSection::Replies => "posts_with_replies",
                ProfileSection::Media => "posts_with_media",
                _ => unreachable!(),
            };
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_author_feed(
                    get_author_feed::ParametersData {
                        actor,
                        cursor: None,
                        filter: Some(filter.to_owned()),
                        include_pins: Some(true),
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Posts(output.feed.clone()))
        }
        ProfileSection::Likes => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_actor_likes(
                    get_actor_likes::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Posts(output.feed.clone()))
        }
        ProfileSection::Feeds => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_actor_feeds(
                    get_actor_feeds::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .feeds
                    .iter()
                    .map(|feed| ProfileListItem {
                        title: feed.display_name.clone(),
                        subtitle: feed.description.clone().unwrap_or_default(),
                        url: at_uri_web_url(&feed.uri, "feed").unwrap_or_default(),
                    })
                    .collect(),
            ))
        }
        ProfileSection::Lists => {
            let output = agent
                .api
                .app
                .bsky
                .graph
                .get_lists(
                    get_lists::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                        purposes: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .lists
                    .iter()
                    .map(|list| ProfileListItem {
                        title: list.name.clone(),
                        subtitle: list.description.clone().unwrap_or_else(|| {
                            format!("{} members", list.list_item_count.unwrap_or(0))
                        }),
                        url: at_uri_web_url(&list.uri, "lists").unwrap_or_default(),
                    })
                    .collect(),
            ))
        }
        ProfileSection::StarterPacks => {
            let output = agent
                .api
                .app
                .bsky
                .graph
                .get_actor_starter_packs(
                    get_actor_starter_packs::ParametersData {
                        actor,
                        cursor: None,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(ProfileContent::Items(
                output
                    .starter_packs
                    .iter()
                    .map(|pack| {
                        let record =
                            starterpack::Record::try_from_unknown(pack.record.clone()).ok();
                        ProfileListItem {
                            title: record
                                .as_ref()
                                .map(|record| record.name.clone())
                                .unwrap_or_else(|| "Starter Pack".to_owned()),
                            subtitle: record
                                .and_then(|record| record.description.clone())
                                .unwrap_or_else(|| {
                                    format!("{} members", pack.list_item_count.unwrap_or(0))
                                }),
                            url: starter_pack_url(&pack.uri, pack.creator.did.as_str())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
            ))
        }
    }
}

pub async fn toggle_follow(
    agent: &BskyAgent,
    profile: &atrium_api::app::bsky::actor::defs::ProfileViewDetailedData,
) -> Result<Option<String>> {
    if let Some(uri) = profile
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.following.as_ref())
    {
        agent.delete_record(uri).await?;
        Ok(None)
    } else {
        let output = agent
            .create_record(follow::RecordData {
                created_at: Datetime::now(),
                subject: profile.did.clone(),
            })
            .await?;
        Ok(Some(output.uri.clone()))
    }
}

fn at_uri_parts(uri: &str) -> Option<(&str, &str)> {
    let mut parts = uri.strip_prefix("at://")?.split('/');
    let did = parts.next()?;
    let _collection = parts.next()?;
    let rkey = parts.next()?;
    Some((did, rkey))
}

fn at_uri_web_url(uri: &str, kind: &str) -> Option<String> {
    let (did, rkey) = at_uri_parts(uri)?;
    Some(format!("https://bsky.app/profile/{did}/{kind}/{rkey}"))
}

fn starter_pack_url(uri: &str, creator: &str) -> Option<String> {
    let (_, rkey) = at_uri_parts(uri)?;
    Some(format!("https://bsky.app/starter-pack/{creator}/{rkey}"))
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

pub async fn search_users(agent: &BskyAgent, query: String) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .search_actors(
            search_actors::ParametersData {
                cursor: None,
                limit: None,
                q: Some(query),
                term: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .actors
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn followers(agent: &BskyAgent, actor: AtIdentifier) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_followers(
            get_followers::ParametersData {
                actor,
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .followers
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
}

pub async fn follows(agent: &BskyAgent, actor: AtIdentifier) -> Result<Vec<InteractionItem>> {
    let output = agent
        .api
        .app
        .bsky
        .graph
        .get_follows(
            get_follows::ParametersData {
                actor,
                cursor: None,
                limit: None,
            }
            .into(),
        )
        .await?;
    Ok(output
        .follows
        .iter()
        .map(|profile| profile_interaction(profile))
        .collect())
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

pub fn post_facets(post: &defs::PostViewData) -> Vec<PostFacet> {
    let Ok(record) = post::Record::try_from_unknown(post.record.clone()) else {
        return Vec::new();
    };
    facets_from_record(&record)
}

pub fn feed_context_lines(feed: &defs::FeedViewPostData) -> Vec<String> {
    let mut lines = Vec::new();
    match &feed.reason {
        Some(Union::Refs(defs::FeedViewPostReasonRefs::ReasonRepost(reason))) => {
            lines.push(format!("🔁 Reposted by @{}", reason.by.handle.as_str()))
        }
        Some(Union::Refs(defs::FeedViewPostReasonRefs::ReasonPin(_))) => {
            lines.push("📌 Pinned post".to_owned());
        }
        Some(Union::Unknown(_)) | None => {}
    }
    if feed
        .post
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.pinned)
        .unwrap_or(false)
        && !lines.iter().any(|line| line.contains("Pinned"))
    {
        lines.push("📌 Pinned post".to_owned());
    }
    if let Some(reply) = &feed.reply {
        let context = match &reply.parent {
            Union::Refs(defs::ReplyRefParentRefs::PostView(parent)) => {
                format!("↪ Reply to @{}", parent.author.handle.as_str())
            }
            Union::Refs(defs::ReplyRefParentRefs::NotFoundPost(_)) => {
                "↪ Reply to [unavailable post]".to_owned()
            }
            Union::Refs(defs::ReplyRefParentRefs::BlockedPost(_)) => {
                "↪ Reply to [blocked post]".to_owned()
            }
            Union::Unknown(_) => "↪ Reply to [unsupported post]".to_owned(),
        };
        lines.push(context);
    }
    lines
}

pub fn verification_badge(post: &defs::PostViewData) -> &'static str {
    let Some(verification) = &post.author.verification else {
        return "";
    };
    if verification.trusted_verifier_status == "valid" {
        "◆"
    } else if verification.verified_status == "valid" {
        "✓"
    } else {
        ""
    }
}

fn facets_from_record(record: &post::RecordData) -> Vec<PostFacet> {
    record
        .facets
        .iter()
        .flatten()
        .flat_map(|facet| {
            let label = record
                .text
                .get(facet.index.byte_start..facet.index.byte_end)
                .unwrap_or_default()
                .to_owned();
            facet.features.iter().filter_map(move |feature| {
                let (kind, url) = match feature {
                    Union::Refs(facet::MainFeaturesItem::Link(link)) => ("URL", link.uri.clone()),
                    Union::Refs(facet::MainFeaturesItem::Mention(mention)) => (
                        "Mention",
                        format!("https://bsky.app/profile/{}", mention.did.as_str()),
                    ),
                    Union::Refs(facet::MainFeaturesItem::Tag(tag)) => {
                        let mut url = reqwest::Url::parse("https://bsky.app/hashtag/").ok()?;
                        url.path_segments_mut().ok()?.push(&tag.tag);
                        ("Hashtag", url.into())
                    }
                    Union::Unknown(_) => return None,
                };
                Some(PostFacet {
                    label: if label.is_empty() {
                        url.clone()
                    } else {
                        label.clone()
                    },
                    kind,
                    url,
                })
            })
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facets_use_utf8_byte_ranges_and_build_safe_targets() {
        let text = "日本 https://example.com #青".to_owned();
        let link_start = text.find("https").expect("link is present");
        let link_end = link_start + "https://example.com".len();
        let tag_start = text.find("#青").expect("tag is present");
        let record = post::RecordData {
            created_at: Datetime::now(),
            embed: None,
            entities: None,
            facets: Some(vec![
                facet::MainData {
                    features: vec![Union::Refs(facet::MainFeaturesItem::Link(Box::new(
                        facet::LinkData {
                            uri: "https://example.com/full".to_owned(),
                        }
                        .into(),
                    )))],
                    index: facet::ByteSliceData {
                        byte_start: link_start,
                        byte_end: link_end,
                    }
                    .into(),
                }
                .into(),
                facet::MainData {
                    features: vec![Union::Refs(facet::MainFeaturesItem::Tag(Box::new(
                        facet::TagData {
                            tag: "青".to_owned(),
                        }
                        .into(),
                    )))],
                    index: facet::ByteSliceData {
                        byte_start: tag_start,
                        byte_end: text.len(),
                    }
                    .into(),
                }
                .into(),
            ]),
            labels: None,
            langs: None,
            reply: None,
            tags: None,
            text,
        };

        let facets = facets_from_record(&record);
        assert_eq!(facets[0].label, "https://example.com");
        assert_eq!(facets[0].url, "https://example.com/full");
        assert_eq!(facets[1].label, "#青");
        assert!(facets[1].url.ends_with("/%E9%9D%92"));
    }

    #[test]
    fn invalid_facet_byte_range_does_not_panic() {
        let record = post::RecordData {
            created_at: Datetime::now(),
            embed: None,
            entities: None,
            facets: Some(vec![facet::MainData {
                features: vec![Union::Refs(facet::MainFeaturesItem::Link(Box::new(
                    facet::LinkData {
                        uri: "https://example.com".to_owned(),
                    }
                    .into(),
                )))],
                index: facet::ByteSliceData {
                    byte_start: 1,
                    byte_end: 999,
                }
                .into(),
            }
            .into()]),
            labels: None,
            langs: None,
            reply: None,
            tags: None,
            text: "青".to_owned(),
        };

        let facets = facets_from_record(&record);
        assert_eq!(facets[0].label, "https://example.com");
    }

    #[test]
    fn profile_resource_urls_are_built_from_at_uris() {
        let uri = "at://did:plc:example/app.bsky.feed.generator/abc123";
        assert_eq!(
            at_uri_web_url(uri, "feed").as_deref(),
            Some("https://bsky.app/profile/did:plc:example/feed/abc123")
        );
        assert_eq!(
            starter_pack_url(uri, "did:plc:creator").as_deref(),
            Some("https://bsky.app/starter-pack/did:plc:creator/abc123")
        );
        assert!(at_uri_web_url("not-an-at-uri", "feed").is_none());
    }
}
