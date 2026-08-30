//! timeline effects.

use super::*;

impl IoAsyncHandler {
    pub(super) async fn initialize(&mut self) -> Result<()> {
        let config = AppConfig::load()?;
        let account = config.active_account();
        let identifier = account.identifier.clone();
        let credentials = tokio::task::spawn_blocking(move || AuthCredentials::load(&identifier))
            .await
            .wrap_err("credential lookup task failed")??;
        let (agent, session) = bsky::agent_with_session(
            &account.service_url,
            &account.identifier,
            credentials.app_password(),
        )
        .await?;
        let moderation = bsky::moderation_preferences(&agent).await?;
        self.emit(EffectMessage::Initialized {
            agent,
            handle: session.handle.clone(),
            did: session.did.clone(),
            moderation,
            ui_config: config.ui.clone(),
        })
        .await;
        if self.load_feed_catalog(false).await.is_err() {
            self.emit(EffectMessage::FeedCatalogLoaded {
                catalog: vec![
                    crate::app::feed::FeedDescriptor::following(),
                    crate::app::feed::FeedDescriptor::discover(),
                ],
                open: false,
            })
            .await;
        }
        self.load_timeline(TimelineEvent::Load).await
    }

    pub(super) async fn load_timeline(&mut self, event: TimelineEvent) -> Result<()> {
        let current = self.state().get_tl_current_cursor_index();
        let cursors = self.state().get_cursors();
        let target = match event {
            TimelineEvent::Load => 0,
            TimelineEvent::Reload => current,
            TimelineEvent::Prev if current > 0 => current - 1,
            TimelineEvent::Prev => return Ok(()),
            TimelineEvent::Next => current + 1,
        };
        let cursor = if matches!(event, TimelineEvent::Load) {
            None
        } else {
            cursors.get(target).cloned().flatten()
        };
        if matches!(event, TimelineEvent::Next) && cursor.is_none() {
            return Ok(());
        }

        let active_feed = self.state().get_active_feed();
        let old_timeline = self.state().get_timeline().unwrap_or_default();
        let old_position = self.state().get_tl_list_position();
        let agent = self.agent().await?;
        let timeline = agent.load_timeline(&active_feed, cursor.clone()).await?;
        let moderation = self.state().moderation();
        let image_urls = timeline
            .feed
            .iter()
            .flat_map(|feed| bsky::post_image_urls(&feed.post, &moderation))
            .collect::<Vec<_>>();
        let page_cursors = updated_cursors(
            if matches!(event, TimelineEvent::Load) {
                vec![None]
            } else {
                cursors
            },
            target,
            cursor,
            timeline.cursor.clone(),
        );

        let position = if matches!(event, TimelineEvent::Reload) {
            old_timeline
                .get(old_position)
                .and_then(|selected| {
                    timeline
                        .feed
                        .iter()
                        .position(|feed| feed.post.uri == selected.post.uri)
                })
                .unwrap_or(old_position)
        } else {
            0
        };
        let new_count = if matches!(event, TimelineEvent::Reload) && current == 0 {
            count_new_posts(&old_timeline, &timeline.feed)
        } else {
            0
        };
        self.emit(EffectMessage::TimelineLoaded {
            posts: timeline.feed,
            position,
            new_count,
            cursors: page_cursors,
            cursor_index: target,
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn load_feed_catalog(&mut self, open: bool) -> Result<()> {
        let catalog = bsky::feed_catalog(self.agent().await?.as_ref()).await?;
        self.emit(EffectMessage::FeedCatalogLoaded { catalog, open })
            .await;
        Ok(())
    }

    pub(super) async fn search_feeds(&mut self, query: String) -> Result<()> {
        let results = bsky::search_feeds(self.agent().await?.as_ref(), query).await?;
        self.emit(EffectMessage::FeedSearchLoaded(results)).await;
        Ok(())
    }

    pub(super) async fn select_feed(
        &mut self,
        feed: crate::app::feed::FeedDescriptor,
    ) -> Result<()> {
        self.emit(EffectMessage::FeedActivated(feed)).await;
        let needs_load = self.state().get_timeline().is_none();
        if needs_load {
            self.load_timeline(TimelineEvent::Load).await?;
        }
        Ok(())
    }

    pub(super) async fn toggle_saved_feed(
        &mut self,
        feed: crate::app::feed::FeedDescriptor,
    ) -> Result<()> {
        bsky::toggle_saved_feed(self.agent().await?.as_ref(), &feed).await?;
        self.load_feed_catalog(true).await
    }

    pub(super) async fn delete_post(&mut self, uri: String) -> Result<()> {
        self.agent().await?.delete_record(uri).await?;
        let mode = self.state().get_mode();
        let tab = self.state().get_tab();
        let section = self.state().profile_section;
        match mode {
            Mode::Profile => {
                if let Some(section) = section {
                    self.load_profile_section(section).await?;
                }
            }
            Mode::Thread => {
                self.emit(EffectMessage::ThreadClosed).await;
                self.load_timeline(TimelineEvent::Reload).await?;
            }
            _ if tab == Tab::Search => self.search(SearchEvent::Reload).await?,
            _ => self.load_timeline(TimelineEvent::Reload).await?,
        }
        Ok(())
    }

    pub(super) async fn preview_link(&mut self, url: String) -> Result<()> {
        let preview = bsky::fetch_link_preview(&url).await?;
        self.emit(EffectMessage::ComposerPreviewLoaded(preview))
            .await;
        Ok(())
    }
}
