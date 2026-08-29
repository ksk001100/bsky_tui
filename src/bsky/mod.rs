use std::{convert::TryInto, num::NonZeroU64};

use eyre::{bail, Result};
use unicode_segmentation::UnicodeSegmentation;

use atrium_api::{
    agent::atp_agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::{
        actor::{
            defs::PreferencesItem, get_preferences, get_profile, put_preferences, search_actors,
        },
        embed::{
            defs as embed_defs, external, images, record as embed_record, record::ViewRecordRefs,
            record_with_media, record_with_media::ViewMediaRefs,
        },
        feed::{
            defs, get_actor_feeds, get_actor_likes, get_author_feed, get_feed, get_feed_generators,
            get_likes, get_post_thread, get_quotes, get_reposted_by, get_timeline, post,
            search_posts, threadgate,
        },
        graph::{
            block, follow, get_actor_starter_packs, get_followers, get_follows, get_lists,
            mute_actor, starterpack, unmute_actor,
        },
        notification,
        richtext::facet,
        unspecced::get_popular_feed_generators,
    },
    com::atproto::{admin::defs::RepoRefData, label::defs as label_defs, moderation, repo, server},
    record::KnownRecord,
    types::{
        string::{AtIdentifier, Cid, Datetime, Did, Handle, Nsid, RecordKey},
        Union,
    },
};
use atrium_xrpc_client::reqwest::ReqwestClient;

use bsky_sdk::api::types::TryFromUnknown;
use bsky_sdk::BskyAgent;
use image::GenericImageView;

use crate::app::composer::{PostDraft, ReplyPolicy, ReplyRule};
use crate::app::feed::{FeedDescriptor, FeedKind};
use crate::app::moderation::ModerationPrefs;
use crate::app::profile::{ProfileContent, ProfileListItem, ProfileSection};
use crate::io::ModerationAction;

pub mod feature_services;

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

const DISCOVER_FEED: &str =
    "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot";

pub struct TimelinePage {
    pub feed: Vec<defs::FeedViewPost>,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LinkPreview {
    pub url: String,
    pub title: String,
    pub description: String,
}

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

pub async fn selected_feed_timeline(
    agent: &BskyAgent,
    descriptor: &FeedDescriptor,
    cursor: Option<String>,
) -> Result<TimelinePage> {
    match &descriptor.kind {
        FeedKind::Following => {
            let output = timeline(agent, cursor).await?;
            Ok(TimelinePage {
                feed: output.feed.clone(),
                cursor: output.cursor.clone(),
            })
        }
        FeedKind::Custom(uri) => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_feed(
                    get_feed::ParametersData {
                        cursor,
                        feed: uri.clone(),
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(TimelinePage {
                feed: output.feed.clone(),
                cursor: output.cursor.clone(),
            })
        }
        FeedKind::List(uri) => {
            let output = agent
                .api
                .app
                .bsky
                .feed
                .get_list_feed(
                    atrium_api::app::bsky::feed::get_list_feed::ParametersData {
                        cursor,
                        list: uri.clone(),
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            Ok(TimelinePage {
                feed: output.feed.clone(),
                cursor: output.cursor.clone(),
            })
        }
    }
}

pub async fn feed_catalog(agent: &BskyAgent) -> Result<Vec<FeedDescriptor>> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .get_preferences(get_preferences::ParametersData {}.into())
        .await?;
    let saved = saved_feed_items(&output.preferences);
    let mut uris = vec![DISCOVER_FEED.to_owned()];
    uris.extend(saved.iter().map(|item| item.value.clone()));
    uris.sort();
    uris.dedup();
    let generators = agent
        .api
        .app
        .bsky
        .feed
        .get_feed_generators(get_feed_generators::ParametersData { feeds: uris }.into())
        .await?
        .feeds
        .clone();
    let mut catalog = vec![FeedDescriptor::following()];
    let mut descriptors = generators
        .iter()
        .map(|generator| {
            let saved_item = saved.iter().find(|item| item.value == generator.uri);
            FeedDescriptor {
                id: generator.uri.clone(),
                name: if generator.uri == DISCOVER_FEED {
                    "Discover".to_owned()
                } else {
                    generator.display_name.clone()
                },
                description: generator.description.clone().unwrap_or_default(),
                kind: FeedKind::Custom(generator.uri.clone()),
                saved: saved_item.is_some(),
                pinned: saved_item.is_some_and(|item| item.pinned),
            }
        })
        .collect::<Vec<_>>();
    if !descriptors.iter().any(|feed| feed.id == DISCOVER_FEED) {
        descriptors.push(FeedDescriptor {
            id: DISCOVER_FEED.to_owned(),
            name: "Discover".to_owned(),
            description: "Popular posts selected by Bluesky".to_owned(),
            kind: FeedKind::Custom(DISCOVER_FEED.to_owned()),
            saved: saved.iter().any(|item| item.value == DISCOVER_FEED),
            pinned: saved
                .iter()
                .find(|item| item.value == DISCOVER_FEED)
                .is_some_and(|item| item.pinned),
        });
    }
    descriptors.sort_by_key(|feed| {
        let saved_index = saved
            .iter()
            .position(|item| item.value == feed.id)
            .unwrap_or(usize::MAX);
        (!feed.pinned, saved_index, feed.name.clone())
    });
    catalog.extend(descriptors);
    Ok(catalog)
}

pub async fn search_feeds(agent: &BskyAgent, query: String) -> Result<Vec<FeedDescriptor>> {
    let output = agent
        .api
        .app
        .bsky
        .unspecced
        .get_popular_feed_generators(
            get_popular_feed_generators::ParametersData {
                cursor: None,
                limit: None,
                query: Some(query),
            }
            .into(),
        )
        .await?;
    Ok(output
        .feeds
        .iter()
        .map(|generator| FeedDescriptor {
            id: generator.uri.clone(),
            name: generator.display_name.clone(),
            description: generator.description.clone().unwrap_or_default(),
            kind: FeedKind::Custom(generator.uri.clone()),
            saved: false,
            pinned: false,
        })
        .collect())
}

pub async fn toggle_saved_feed(agent: &BskyAgent, descriptor: &FeedDescriptor) -> Result<()> {
    let output = agent
        .api
        .app
        .bsky
        .actor
        .get_preferences(get_preferences::ParametersData {}.into())
        .await?;
    let mut preferences = output.preferences.clone();
    let mut found_v2 = false;
    for preference in &mut preferences {
        if let Union::Refs(PreferencesItem::SavedFeedsPrefV2(saved)) = preference {
            found_v2 = true;
            if let Some(index) = saved
                .items
                .iter()
                .position(|item| item.value == descriptor.id)
            {
                saved.items.remove(index);
            } else {
                saved.items.push(
                    atrium_api::app::bsky::actor::defs::SavedFeedData {
                        id: descriptor.id.clone(),
                        pinned: false,
                        r#type: "feed".to_owned(),
                        value: descriptor.id.clone(),
                    }
                    .into(),
                );
            }
        }
    }
    if !found_v2 {
        preferences.push(Union::Refs(PreferencesItem::SavedFeedsPrefV2(Box::new(
            atrium_api::app::bsky::actor::defs::SavedFeedsPrefV2Data {
                items: vec![atrium_api::app::bsky::actor::defs::SavedFeedData {
                    id: descriptor.id.clone(),
                    pinned: false,
                    r#type: "feed".to_owned(),
                    value: descriptor.id.clone(),
                }
                .into()],
            }
            .into(),
        ))));
    }
    agent
        .api
        .app
        .bsky
        .actor
        .put_preferences(put_preferences::InputData { preferences }.into())
        .await?;
    Ok(())
}

fn saved_feed_items(
    preferences: &[atrium_api::types::Union<PreferencesItem>],
) -> Vec<atrium_api::app::bsky::actor::defs::SavedFeed> {
    preferences
        .iter()
        .find_map(|preference| match preference {
            Union::Refs(PreferencesItem::SavedFeedsPrefV2(saved)) => Some(saved.items.clone()),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|item| item.r#type == "feed")
        .collect()
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

pub async fn moderate(agent: &BskyAgent, action: ModerationAction) -> Result<()> {
    match action {
        ModerationAction::MuteActor { did, muted } => {
            if muted {
                agent
                    .api
                    .app
                    .bsky
                    .graph
                    .unmute_actor(unmute_actor::InputData { actor: did.into() }.into())
                    .await?;
            } else {
                agent
                    .api
                    .app
                    .bsky
                    .graph
                    .mute_actor(mute_actor::InputData { actor: did.into() }.into())
                    .await?;
            }
        }
        ModerationAction::BlockActor { did, blocking_uri } => {
            if let Some(uri) = blocking_uri {
                agent.delete_record(uri).await?;
            } else {
                agent
                    .create_record(block::RecordData {
                        created_at: Datetime::now(),
                        subject: did,
                    })
                    .await?;
            }
        }
        ModerationAction::ReportActor(did) => {
            agent
                .api
                .com
                .atproto
                .moderation
                .create_report(
                    moderation::create_report::InputData {
                        mod_tool: None,
                        reason: Some("Reported from bsky_tui".to_owned()),
                        reason_type: moderation::defs::REASON_OTHER.to_owned(),
                        subject: Union::Refs(
                            moderation::create_report::InputSubjectRefs::ComAtprotoAdminDefsRepoRef(
                                Box::new(RepoRefData { did }.into()),
                            ),
                        ),
                    }
                    .into(),
                )
                .await?;
        }
        ModerationAction::ReportPost { uri, cid } => {
            agent
                .api
                .com
                .atproto
                .moderation
                .create_report(
                    moderation::create_report::InputData {
                        mod_tool: None,
                        reason: Some("Reported from bsky_tui".to_owned()),
                        reason_type: moderation::defs::REASON_OTHER.to_owned(),
                        subject: Union::Refs(
                            moderation::create_report::InputSubjectRefs::ComAtprotoRepoStrongRefMain(
                                Box::new(repo::strong_ref::MainData { uri, cid }.into()),
                            ),
                        ),
                    }
                    .into(),
                )
                .await?;
        }
    }
    Ok(())
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

async fn create_draft_record(
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

async fn rollback_created_posts(
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

async fn build_embed(
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

async fn upload_images(
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

async fn external_preview(url: &str) -> Result<external::Main> {
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

fn html_tag_content(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find(&format!("<{tag}"))?;
    let content_start = lower[start..].find('>')? + start + 1;
    let end = lower[content_start..].find(&format!("</{tag}>"))? + content_start;
    Some(html[content_start..end].trim().to_owned())
}

fn html_meta_content(html: &str, name: &str) -> Option<String> {
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

async fn create_thread_gate(
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

pub async fn notification_preferences(
    agent: &BskyAgent,
) -> Result<notification::defs::Preferences> {
    Ok(agent
        .api
        .app
        .bsky
        .notification
        .get_preferences(notification::get_preferences::ParametersData {}.into())
        .await?
        .preferences
        .clone())
}

pub async fn put_notification_preferences(
    agent: &BskyAgent,
    preferences: notification::defs::Preferences,
) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .put_preferences_v2(
            notification::put_preferences_v2::InputData {
                chat: Some(preferences.chat.clone()),
                follow: Some(preferences.follow.clone()),
                like: Some(preferences.like.clone()),
                like_via_repost: Some(preferences.like_via_repost.clone()),
                mention: Some(preferences.mention.clone()),
                quote: Some(preferences.quote.clone()),
                reply: Some(preferences.reply.clone()),
                repost: Some(preferences.repost.clone()),
                repost_via_repost: Some(preferences.repost_via_repost.clone()),
                starterpack_joined: Some(preferences.starterpack_joined.clone()),
                subscribed_post: Some(preferences.subscribed_post.clone()),
                unverified: Some(preferences.unverified.clone()),
                verified: Some(preferences.verified.clone()),
            }
            .into(),
        )
        .await?;
    Ok(())
}

pub async fn put_activity_subscription(
    agent: &BskyAgent,
    subject: Did,
    activity: notification::defs::ActivitySubscription,
) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .put_activity_subscription(
            notification::put_activity_subscription::InputData {
                activity_subscription: activity,
                subject,
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
