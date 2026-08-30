//! display Bluesky services.

use super::*;

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
    let Some(embed) = &post.embed else {
        return Vec::new();
    };
    let Union::Refs(embed) = embed else {
        return vec!["[Unsupported content]".to_owned()];
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

pub fn quoted_post(post: &defs::PostViewData) -> Option<(String, Did)> {
    let Union::Refs(embed) = post.embed.as_ref()? else {
        return None;
    };
    let record = match embed {
        defs::PostViewEmbedRefs::AppBskyEmbedRecordView(record) => record,
        defs::PostViewEmbedRefs::AppBskyEmbedRecordWithMediaView(embed) => &embed.record,
        _ => return None,
    };
    match &record.record {
        Union::Refs(ViewRecordRefs::ViewRecord(quoted)) => {
            Some((quoted.uri.clone(), quoted.author.did.clone()))
        }
        _ => None,
    }
}

pub fn post_facets(post: &defs::PostViewData) -> Vec<PostFacet> {
    let Ok(record) = post::Record::try_from_unknown(post.record.clone()) else {
        return Vec::new();
    };
    facets_from_record(&record)
}

pub fn post_text(post: &defs::PostViewData) -> Option<String> {
    post::Record::try_from_unknown(post.record.clone())
        .ok()
        .map(|record| record.text.clone())
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

pub(super) fn facets_from_record(record: &post::RecordData) -> Vec<PostFacet> {
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

pub(super) fn video_lines(video: &atrium_api::app::bsky::embed::video::View) -> Vec<String> {
    vec![
        "Video/GIF (press e to play externally)".to_owned(),
        format!("Alt: {}", video.alt.as_deref().unwrap_or("(not provided)")),
        "Captions: available through the video playlist when provided".to_owned(),
    ]
}

pub(super) fn record_lines(record: &atrium_api::app::bsky::embed::record::View) -> Vec<String> {
    match &record.record {
        Union::Refs(ViewRecordRefs::ViewRecord(quoted)) => {
            let record = post::Record::try_from_unknown(quoted.value.clone()).ok();
            let text = record
                .as_ref()
                .map(|record| record.text.clone())
                .unwrap_or_else(|| "[Quoted record unavailable]".to_owned());
            vec![
                format!(
                    "Quote: {} @{} · {}",
                    quoted.author.display_name.clone().unwrap_or_default(),
                    quoted.author.handle.as_str(),
                    record
                        .as_ref()
                        .map(|record| utils::format_post_datetime(record.created_at.as_str()))
                        .unwrap_or_else(|| "time unavailable".to_owned())
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

pub(super) fn post_attachments(
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

pub(super) fn post_embed_thumbnail(post: &defs::PostViewData) -> Option<String> {
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
