use std::{convert::TryInto, future::Future, num::NonZeroU64, pin::Pin};

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
            get_likes, get_post_thread, get_posts, get_quotes, get_reposted_by, get_timeline, post,
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
use crate::utils;

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

mod session;
pub use session::*;
mod feeds;
pub use feeds::*;
mod social;
pub use social::*;
mod display;
pub use display::*;
mod publishing;
pub use publishing::*;
mod notifications;
pub use notifications::*;
mod reactions;
pub use reactions::*;

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTimelineClient;

    impl TimelineClient for MockTimelineClient {
        fn load_timeline<'a>(
            &'a self,
            _descriptor: &'a FeedDescriptor,
            cursor: Option<String>,
        ) -> Pin<Box<dyn Future<Output = Result<TimelinePage>> + Send + 'a>> {
            Box::pin(async move {
                Ok(TimelinePage {
                    feed: Vec::new(),
                    cursor: cursor.map(|value| format!("next-{value}")),
                })
            })
        }
    }

    #[tokio::test]
    async fn timeline_client_is_mockable_without_network_access() {
        let page = MockTimelineClient
            .load_timeline(&FeedDescriptor::following(), Some("cursor".into()))
            .await
            .expect("mock page");
        assert!(page.feed.is_empty());
        assert_eq!(page.cursor.as_deref(), Some("next-cursor"));
    }

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
