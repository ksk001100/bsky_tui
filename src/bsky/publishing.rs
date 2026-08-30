//! publishing Bluesky services.

use super::*;

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

pub async fn send_drafts(
    agent: &BskyAgent,
    drafts: Vec<PostDraft>,
    initial_reply: Option<post::ReplyRef>,
) -> Result<Vec<repo::strong_ref::MainData>> {
    for draft in &drafts {
        if !draft.text.trim().is_empty() {
            validate_post_text(&draft.text)?;
        }
    }
    let mut created: Vec<(repo::strong_ref::MainData, Option<String>)> = Vec::new();
    let mut root = initial_reply
        .as_ref()
        .map(|reply| repo::strong_ref::MainData {
            uri: reply.root.uri.clone(),
            cid: reply.root.cid.clone(),
        });
    let mut parent = initial_reply
        .as_ref()
        .map(|reply| repo::strong_ref::MainData {
            uri: reply.parent.uri.clone(),
            cid: reply.parent.cid.clone(),
        });

    for draft in drafts {
        let reply = parent.as_ref().map(|current_parent| {
            post::ReplyRefData {
                parent: current_parent.clone().into(),
                root: root
                    .clone()
                    .unwrap_or_else(|| current_parent.clone())
                    .into(),
            }
            .into()
        });
        let (reference, gate_uri) = match create_draft_record(agent, &draft, reply).await {
            Ok(created) => created,
            Err(error) => {
                rollback_created_posts(agent, &created).await;
                return Err(error);
            }
        };
        if root.is_none() {
            root = Some(reference.clone());
        }
        parent = Some(reference.clone());
        created.push((reference, gate_uri));
    }
    Ok(created.into_iter().map(|(post, _)| post).collect())
}

pub(super) async fn create_draft_record(
    agent: &BskyAgent,
    draft: &PostDraft,
    reply: Option<post::ReplyRef>,
) -> Result<(repo::strong_ref::MainData, Option<String>)> {
    let rich_text = bsky_sdk::rich_text::RichText::new_with_detect_facets(&draft.text).await?;
    let embed = build_embed(agent, draft).await?;
    let labels = (!draft.labels.is_empty()).then(|| {
        Union::Refs(post::RecordLabelsRefs::ComAtprotoLabelDefsSelfLabels(
            Box::new(
                label_defs::SelfLabelsData {
                    values: draft
                        .labels
                        .iter()
                        .map(|label| label_defs::SelfLabelData { val: label.clone() }.into())
                        .collect(),
                }
                .into(),
            ),
        ))
    });
    let output = agent
        .create_record(post::RecordData {
            created_at: Datetime::now(),
            embed,
            entities: None,
            facets: rich_text.facets,
            langs: draft.langs.clone(),
            labels,
            tags: None,
            reply,
            text: draft.text.clone(),
        })
        .await?;
    let reference = repo::strong_ref::MainData {
        uri: output.uri.clone(),
        cid: output.cid.clone(),
    };
    match create_thread_gate(agent, &reference.uri, &draft.reply_policy).await {
        Ok(gate_uri) => Ok((reference, gate_uri)),
        Err(error) => {
            let _ = agent.delete_record(&reference.uri).await;
            Err(error)
        }
    }
}

pub(super) async fn rollback_created_posts(
    agent: &BskyAgent,
    created: &[(repo::strong_ref::MainData, Option<String>)],
) {
    for (post, gate) in created.iter().rev() {
        if let Some(gate) = gate {
            let _ = agent.delete_record(gate).await;
        }
        let _ = agent.delete_record(&post.uri).await;
    }
}

pub(super) async fn build_embed(
    agent: &BskyAgent,
    draft: &PostDraft,
) -> Result<Option<Union<post::RecordEmbedRefs>>> {
    let media = if !draft.images.is_empty() {
        Some(Union::Refs(
            record_with_media::MainMediaRefs::AppBskyEmbedImagesMain(Box::new(
                upload_images(agent, &draft.images).await?,
            )),
        ))
    } else if let Some(url) = &draft.link {
        Some(Union::Refs(
            record_with_media::MainMediaRefs::AppBskyEmbedExternalMain(Box::new(
                external_preview(url).await?,
            )),
        ))
    } else {
        None
    };
    let quote = draft.quote.as_ref().map(|reference| {
        embed_record::MainData {
            record: reference.clone().into(),
        }
        .into()
    });

    Ok(match (media, quote) {
        (Some(media), Some(record)) => Some(Union::Refs(
            post::RecordEmbedRefs::AppBskyEmbedRecordWithMediaMain(Box::new(
                record_with_media::MainData { media, record }.into(),
            )),
        )),
        (
            Some(Union::Refs(record_with_media::MainMediaRefs::AppBskyEmbedImagesMain(images))),
            None,
        ) => Some(Union::Refs(post::RecordEmbedRefs::AppBskyEmbedImagesMain(
            images,
        ))),
        (
            Some(Union::Refs(record_with_media::MainMediaRefs::AppBskyEmbedExternalMain(external))),
            None,
        ) => Some(Union::Refs(
            post::RecordEmbedRefs::AppBskyEmbedExternalMain(external),
        )),
        (Some(Union::Refs(_)), None) | (Some(Union::Unknown(_)), None) => {
            bail!("unsupported composer media")
        }
        (None, Some(record)) => Some(Union::Refs(post::RecordEmbedRefs::AppBskyEmbedRecordMain(
            Box::new(record),
        ))),
        (None, None) => None,
    })
}

pub(super) async fn upload_images(
    agent: &BskyAgent,
    specs: &[crate::app::composer::ImageSpec],
) -> Result<images::Main> {
    let mut uploaded = Vec::new();
    for spec in specs {
        let bytes = tokio::fs::read(&spec.path).await?;
        if bytes.len() > 1_000_000 {
            bail!("image {} exceeds the 1 MB upload limit", spec.path);
        }
        let decoded = image::load_from_memory(&bytes)?;
        let (width, height) = decoded.dimensions();
        let width = NonZeroU64::new(width.into()).ok_or_else(|| eyre::eyre!("zero-width image"))?;
        let height =
            NonZeroU64::new(height.into()).ok_or_else(|| eyre::eyre!("zero-height image"))?;
        let blob = agent
            .api
            .com
            .atproto
            .repo
            .upload_blob(bytes)
            .await?
            .blob
            .clone();
        uploaded.push(
            images::ImageData {
                alt: spec.alt.clone(),
                aspect_ratio: Some(embed_defs::AspectRatioData { width, height }.into()),
                image: blob,
            }
            .into(),
        );
    }
    Ok(images::MainData { images: uploaded }.into())
}

pub(super) async fn external_preview(url: &str) -> Result<external::Main> {
    let preview = fetch_link_preview(url).await?;
    Ok(external::MainData {
        external: external::ExternalData {
            description: preview.description,
            thumb: None,
            title: preview.title,
            uri: preview.url,
        }
        .into(),
    }
    .into())
}

pub async fn fetch_link_preview(url: &str) -> Result<LinkPreview> {
    let parsed = reqwest::Url::parse(url)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("external card URL must use http or https");
    }
    let response = reqwest::get(parsed.clone()).await?;
    if response
        .content_length()
        .is_some_and(|length| length > 2_000_000)
    {
        bail!("external preview page is too large");
    }
    let bytes = response.bytes().await?;
    if bytes.len() > 2_000_000 {
        bail!("external preview page is too large");
    }
    let html = String::from_utf8_lossy(&bytes);
    let title = html_tag_content(&html, "title").unwrap_or_else(|| parsed.to_string());
    let description = html_meta_content(&html, "description").unwrap_or_default();
    Ok(LinkPreview {
        url: parsed.into(),
        title,
        description,
    })
}

pub(super) fn html_tag_content(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find(&format!("<{tag}"))?;
    let content_start = lower[start..].find('>')? + start + 1;
    let end = lower[content_start..].find(&format!("</{tag}>"))? + content_start;
    Some(html[content_start..end].trim().to_owned())
}

pub(super) fn html_meta_content(html: &str, name: &str) -> Option<String> {
    html.split('<').find_map(|fragment| {
        let lower = fragment.to_ascii_lowercase();
        if !lower.starts_with("meta") || !lower.contains(&format!("name=\"{name}\"")) {
            return None;
        }
        let content = lower.find("content=\"")? + "content=\"".len();
        let end = fragment[content..].find('"')? + content;
        Some(fragment[content..end].to_owned())
    })
}

pub(super) async fn create_thread_gate(
    agent: &BskyAgent,
    post_uri: &str,
    policy: &ReplyPolicy,
) -> Result<Option<String>> {
    let allow = match policy {
        ReplyPolicy::Everyone => return Ok(None),
        ReplyPolicy::Nobody => Some(Vec::new()),
        ReplyPolicy::Rules(rules) => Some(
            rules
                .iter()
                .map(|rule| match rule {
                    ReplyRule::Followers => Union::Refs(threadgate::RecordAllowItem::FollowerRule(
                        Box::new(threadgate::FollowerRuleData {}.into()),
                    )),
                    ReplyRule::Following => {
                        Union::Refs(threadgate::RecordAllowItem::FollowingRule(Box::new(
                            threadgate::FollowingRuleData {}.into(),
                        )))
                    }
                    ReplyRule::Mentioned => Union::Refs(threadgate::RecordAllowItem::MentionRule(
                        Box::new(threadgate::MentionRuleData {}.into()),
                    )),
                })
                .collect(),
        ),
    };
    let output = agent
        .create_record(threadgate::RecordData {
            allow,
            created_at: Datetime::now(),
            hidden_replies: None,
            post: post_uri.to_owned(),
        })
        .await?;
    Ok(Some(output.uri.clone()))
}
