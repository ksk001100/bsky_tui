//! feeds Bluesky services.

use super::*;

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
        FeedKind::Bookmarks => {
            let output = agent
                .api
                .app
                .bsky
                .bookmark
                .get_bookmarks(
                    atrium_api::app::bsky::bookmark::get_bookmarks::ParametersData {
                        cursor,
                        limit: None,
                    }
                    .into(),
                )
                .await?;
            let feed = output
                .bookmarks
                .iter()
                .filter_map(|bookmark| match &bookmark.item {
                    Union::Refs(
                        atrium_api::app::bsky::bookmark::defs::BookmarkViewItemRefs::AppBskyFeedDefsPostView(post),
                    ) => Some(
                        defs::FeedViewPostData {
                            feed_context: None,
                            post: (**post).clone(),
                            reason: None,
                            reply: None,
                            req_id: None,
                        }
                        .into(),
                    ),
                    Union::Refs(_) | Union::Unknown(_) => None,
                })
                .collect();
            Ok(TimelinePage {
                feed,
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

pub trait TimelineClient: Send + Sync {
    fn load_timeline<'a>(
        &'a self,
        descriptor: &'a FeedDescriptor,
        cursor: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<TimelinePage>> + Send + 'a>>;
}

impl TimelineClient for BskyAgent {
    fn load_timeline<'a>(
        &'a self,
        descriptor: &'a FeedDescriptor,
        cursor: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<TimelinePage>> + Send + 'a>> {
        Box::pin(selected_feed_timeline(self, descriptor, cursor))
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
    let mut catalog = vec![FeedDescriptor::following(), FeedDescriptor::bookmarks()];
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

pub(super) fn saved_feed_items(
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
