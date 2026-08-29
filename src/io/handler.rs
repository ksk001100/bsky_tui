use std::sync::Arc;

use atrium_api::{app::bsky::feed::post::ReplyRefData, com::atproto::repo::strong_ref};
use bsky_sdk::BskyAgent;
use eyre::{eyre, Result, WrapErr};
use tui_input::Input;

use super::{
    FeatureEvent, InteractionKind, IoEvent, NotificationEvent, SearchEvent, TimelineEvent,
};
use crate::{
    app::{
        auth::AuthCredentials, config::AppConfig, profile::ProfileSection, profile::ProfileState,
        state::Mode, state::Tab, App,
    },
    bsky,
};

pub struct IoAsyncHandler {
    app: Arc<tokio::sync::Mutex<App>>,
}

impl IoAsyncHandler {
    pub fn new(app: Arc<tokio::sync::Mutex<App>>) -> Self {
        Self { app }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        let operation = operation_name(&io_event);
        let result = match io_event {
            IoEvent::Initialize => self.do_initialize().await,
            IoEvent::LoadTimeline(action) => self.do_load_timeline(action).await,
            IoEvent::SendPost => self.do_send_post().await,
            IoEvent::LoadNotifications(action) => self.do_load_notifications(action).await,
            IoEvent::LoadNotificationSettings(subject, handle) => {
                self.do_load_notification_settings(subject, handle).await
            }
            IoEvent::SaveNotificationPreferences(preferences) => {
                self.do_save_notification_preferences(*preferences).await
            }
            IoEvent::SaveActivitySubscription { subject, activity } => {
                self.do_save_activity_subscription(subject, activity).await
            }
            IoEvent::ToggleNotificationFollow(subject) => {
                self.do_toggle_notification_follow(subject).await
            }
            IoEvent::LikeNotificationAuthor(subject) => {
                self.do_like_notification_author(subject).await
            }
            IoEvent::Like => self.do_like().await,
            IoEvent::Repost => self.do_repost().await,
            IoEvent::Reply => self.do_reply().await,
            IoEvent::Search(action) => self.do_search(action).await,
            IoEvent::SearchLike => self.do_search_like().await,
            IoEvent::SearchRepost => self.do_search_repost().await,
            IoEvent::SearchReply => self.do_search_reply().await,
            IoEvent::LoadThread(uri) => self.do_load_thread(uri).await,
            IoEvent::LoadInteractions(kind, uri, cid) => {
                self.do_load_interactions(kind, uri, cid).await
            }
            IoEvent::SearchUsers(query) => self.do_search_users(query).await,
            IoEvent::LoadProfile(actor) => self.do_load_profile(actor).await,
            IoEvent::LoadProfileSection(section) => self.do_load_profile_section(section).await,
            IoEvent::ToggleFollow => self.do_toggle_follow().await,
            IoEvent::LoadConnections(kind, actor) => self.do_load_connections(kind, actor).await,
            IoEvent::Moderate(action) => self.do_moderate(action).await,
            IoEvent::LoadFeedCatalog => self.do_load_feed_catalog(true).await,
            IoEvent::SearchFeeds(query) => self.do_search_feeds(query).await,
            IoEvent::SelectFeed(feed) => self.do_select_feed(feed).await,
            IoEvent::ToggleSavedFeed(feed) => self.do_toggle_saved_feed(feed).await,
            IoEvent::DeletePost(uri) => self.do_delete_post(uri).await,
            IoEvent::PreviewLink(url) => self.do_preview_link(url).await,
            IoEvent::Feature(event) => self.handle_feature_event(event).await,
        };

        let mut app = self.app.lock().await;
        app.loaded();
        match result {
            Ok(()) => app.clear_error(),
            Err(error) => app.set_error(user_error(operation, &error)),
        }
    }

    async fn agent(&self) -> Result<Arc<BskyAgent>> {
        self.app
            .lock()
            .await
            .state
            .get_agent()
            .ok_or_else(|| eyre!("client is not initialized"))
    }

    async fn do_initialize(&mut self) -> Result<()> {
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
        {
            let mut app = self.app.lock().await;
            app.initialized(
                agent,
                session.handle.clone(),
                session.did.clone(),
                moderation,
                config.ui.clone(),
            );
        }
        if self.do_load_feed_catalog(false).await.is_err() {
            self.app.lock().await.set_feed_catalog(
                vec![
                    crate::app::feed::FeedDescriptor::following(),
                    crate::app::feed::FeedDescriptor::discover(),
                ],
                false,
            );
        }
        self.do_load_timeline(TimelineEvent::Load).await
    }

    async fn do_load_timeline(&mut self, event: TimelineEvent) -> Result<()> {
        let (current, cursors) = {
            let app = self.app.lock().await;
            (
                app.state.get_tl_current_cursor_index(),
                app.state.get_cursors(),
            )
        };
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

        let active_feed = self.app.lock().await.state.get_active_feed();
        let old_timeline = self
            .app
            .lock()
            .await
            .state
            .get_timeline()
            .unwrap_or_default();
        let old_position = self.app.lock().await.state.get_tl_list_position();
        let timeline = bsky::selected_feed_timeline(
            self.agent().await?.as_ref(),
            &active_feed,
            cursor.clone(),
        )
        .await?;
        let moderation = self.app.lock().await.state.moderation();
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

        let mut app = self.app.lock().await;
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
        app.state
            .set_timeline_preserving_position(Some(timeline.feed.clone()), position);
        app.state.set_active_feed_new_count(new_count);
        app.state.set_cursors(page_cursors);
        app.state.set_tl_current_cursor_index(target);
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_load_feed_catalog(&mut self, open: bool) -> Result<()> {
        let catalog = bsky::feed_catalog(self.agent().await?.as_ref()).await?;
        self.app.lock().await.set_feed_catalog(catalog, open);
        Ok(())
    }

    async fn do_search_feeds(&mut self, query: String) -> Result<()> {
        let results = bsky::search_feeds(self.agent().await?.as_ref(), query).await?;
        self.app.lock().await.set_feed_search_results(results);
        Ok(())
    }

    async fn do_select_feed(&mut self, feed: crate::app::feed::FeedDescriptor) -> Result<()> {
        let needs_load = {
            let mut app = self.app.lock().await;
            app.state.activate_feed(feed);
            app.state.get_timeline().is_none()
        };
        if needs_load {
            self.do_load_timeline(TimelineEvent::Load).await?;
        }
        Ok(())
    }

    async fn do_toggle_saved_feed(&mut self, feed: crate::app::feed::FeedDescriptor) -> Result<()> {
        bsky::toggle_saved_feed(self.agent().await?.as_ref(), &feed).await?;
        self.do_load_feed_catalog(true).await
    }

    async fn do_delete_post(&mut self, uri: String) -> Result<()> {
        self.agent().await?.delete_record(uri).await?;
        let (mode, tab, section) = {
            let app = self.app.lock().await;
            (
                app.state.get_mode(),
                app.state.get_tab(),
                app.state.get_profile().map(|profile| profile.section),
            )
        };
        match mode {
            Mode::Profile => {
                if let Some(section) = section {
                    self.do_load_profile_section(section).await?;
                }
            }
            Mode::Thread => {
                self.app.lock().await.state.close_thread();
                self.do_load_timeline(TimelineEvent::Reload).await?;
            }
            _ if tab == Tab::Search => self.do_search(SearchEvent::Reload).await?,
            _ => self.do_load_timeline(TimelineEvent::Reload).await?,
        }
        Ok(())
    }

    async fn do_preview_link(&mut self, url: String) -> Result<()> {
        let preview = bsky::fetch_link_preview(&url).await?;
        self.app.lock().await.set_composer_preview(preview);
        Ok(())
    }

    async fn do_load_thread(&mut self, uri: String) -> Result<()> {
        let output = bsky::post_thread(self.agent().await?.as_ref(), uri.clone()).await?;
        let entries = crate::app::thread::flatten(&output, &uri);
        let moderation = self.app.lock().await.state.moderation();
        let image_urls = entries
            .iter()
            .filter_map(crate::app::thread::ThreadEntry::post)
            .flat_map(|post| bsky::post_image_urls(post, &moderation))
            .collect::<Vec<_>>();
        let mut app = self.app.lock().await;
        app.state.set_thread(entries);
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_load_interactions(
        &mut self,
        kind: InteractionKind,
        uri: String,
        cid: atrium_api::types::string::Cid,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let items = match kind {
            InteractionKind::Likes => bsky::post_likes(agent.as_ref(), uri, cid).await?,
            InteractionKind::Reposts => bsky::post_reposts(agent.as_ref(), uri, cid).await?,
            InteractionKind::Quotes => bsky::post_quotes(agent.as_ref(), uri, cid).await?,
            InteractionKind::Users | InteractionKind::Followers | InteractionKind::Follows => {
                return Err(eyre!("invalid post interaction type"));
            }
        };
        self.app.lock().await.set_interactions(kind, items);
        Ok(())
    }

    async fn do_search_users(&mut self, query: String) -> Result<()> {
        let items = bsky::search_users(self.agent().await?.as_ref(), query).await?;
        let mut app = self.app.lock().await;
        app.state.set_mode(Mode::Normal);
        app.state.set_input(Input::default());
        app.set_interactions(InteractionKind::Users, items);
        Ok(())
    }

    async fn do_load_profile(
        &mut self,
        actor: atrium_api::types::string::AtIdentifier,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let details = bsky::profile(agent.as_ref(), actor).await?;
        let content = bsky::profile_content(
            agent.as_ref(),
            details.did.clone().into(),
            ProfileSection::Posts,
        )
        .await?;
        let moderation = self.app.lock().await.state.moderation();
        let image_urls = vec![details.avatar.clone(), details.banner.clone()]
            .into_iter()
            .flatten()
            .chain(match &content {
                crate::app::profile::ProfileContent::Posts(posts) => posts
                    .iter()
                    .flat_map(|feed| bsky::post_image_urls(&feed.post, &moderation))
                    .collect::<Vec<_>>(),
                crate::app::profile::ProfileContent::Items(_) => Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut app = self.app.lock().await;
        app.state.open_profile(ProfileState::new(details, content));
        app.set_interactions_closed();
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_load_profile_section(&mut self, section: ProfileSection) -> Result<()> {
        let actor = self
            .app
            .lock()
            .await
            .state
            .get_profile()
            .map(|profile| profile.details.did.clone().into())
            .ok_or_else(|| eyre!("no profile is open"))?;
        let content = bsky::profile_content(self.agent().await?.as_ref(), actor, section).await?;
        let moderation = self.app.lock().await.state.moderation();
        let image_urls = match &content {
            crate::app::profile::ProfileContent::Posts(posts) => posts
                .iter()
                .flat_map(|feed| bsky::post_image_urls(&feed.post, &moderation))
                .collect(),
            crate::app::profile::ProfileContent::Items(_) => Vec::new(),
        };
        let mut app = self.app.lock().await;
        app.state.set_profile_content(section, content);
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_toggle_follow(&mut self) -> Result<()> {
        let details = self
            .app
            .lock()
            .await
            .state
            .get_profile()
            .map(|profile| profile.details)
            .ok_or_else(|| eyre!("no profile is open"))?;
        if details.did == self.app.lock().await.state.get_did() {
            return Err(eyre!("you cannot follow your own account"));
        }
        let agent = self.agent().await?;
        let was_following = details
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.following.as_ref())
            .is_some();
        let following_uri = bsky::toggle_follow(agent.as_ref(), &details).await?;
        let mut refreshed = bsky::profile(agent.as_ref(), details.did.clone().into()).await?;
        refreshed.followers_count = Some(if was_following {
            details.followers_count.unwrap_or(1).saturating_sub(1)
        } else {
            details.followers_count.unwrap_or(0).saturating_add(1)
        });
        if let Some(viewer) = &mut refreshed.viewer {
            viewer.following = following_uri;
        }
        let mut app = self.app.lock().await;
        if let Some(mut profile) = app.state.get_profile() {
            profile.details = refreshed;
            app.state.open_profile(profile);
        }
        Ok(())
    }

    async fn do_load_connections(
        &mut self,
        kind: InteractionKind,
        actor: atrium_api::types::string::AtIdentifier,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let items = match kind {
            InteractionKind::Followers => bsky::followers(agent.as_ref(), actor).await?,
            InteractionKind::Follows => bsky::follows(agent.as_ref(), actor).await?,
            _ => return Err(eyre!("invalid social graph list type")),
        };
        self.app.lock().await.set_interactions(kind, items);
        Ok(())
    }

    async fn do_moderate(&mut self, action: crate::io::ModerationAction) -> Result<()> {
        let actor_change = match &action {
            crate::io::ModerationAction::MuteActor { did, .. }
            | crate::io::ModerationAction::BlockActor { did, .. } => Some(did.clone()),
            _ => None,
        };
        bsky::moderate(self.agent().await?.as_ref(), action).await?;
        if let Some(did) = actor_change {
            let (mode, profile) = {
                let app = self.app.lock().await;
                (app.state.get_mode(), app.state.get_profile())
            };
            if let Some(mut profile) =
                profile.filter(|profile| mode == Mode::Profile && profile.details.did == did)
            {
                profile.details = bsky::profile(self.agent().await?.as_ref(), did.into()).await?;
                self.app.lock().await.state.open_profile(profile);
            } else {
                if mode == Mode::Thread {
                    self.app.lock().await.state.close_thread();
                }
                let tab = self.app.lock().await.state.get_tab();
                if tab == Tab::Search {
                    self.do_search(SearchEvent::Reload).await?;
                } else {
                    self.do_load_timeline(TimelineEvent::Reload).await?;
                }
            }
        }
        Ok(())
    }

    async fn do_send_post(&mut self) -> Result<()> {
        let text = {
            let app = self.app.lock().await;
            app.state.get_input().value().to_string()
        };
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(self.agent().await?.as_ref(), drafts, None).await?;
        self.finish_composer().await;
        self.do_load_timeline(TimelineEvent::Reload).await
    }

    async fn do_load_notifications(&mut self, event: NotificationEvent) -> Result<()> {
        let (current, cursors) = {
            let app = self.app.lock().await;
            (
                app.state.get_notifications_current_cursor_index(),
                app.state.get_notification_cursors(),
            )
        };
        let target = match event {
            NotificationEvent::Load => 0,
            NotificationEvent::Reload => current,
            NotificationEvent::Prev if current > 0 => current - 1,
            NotificationEvent::Prev => return Ok(()),
            NotificationEvent::Next => current + 1,
        };
        let cursor = if matches!(event, NotificationEvent::Load) {
            None
        } else {
            cursors.get(target).cloned().flatten()
        };
        if matches!(event, NotificationEvent::Next) && cursor.is_none() {
            return Ok(());
        }
        let agent = self.agent().await?;
        let notifications = bsky::notifications(agent.as_ref(), cursor.clone()).await?;
        bsky::update_seen(agent.as_ref()).await?;
        let page_cursors = updated_cursors(
            if matches!(event, NotificationEvent::Load) {
                vec![None]
            } else {
                cursors
            },
            target,
            cursor,
            notifications.cursor.clone(),
        );
        let mut app = self.app.lock().await;
        app.state
            .set_notifications(Some(notifications.notifications.clone()));
        app.state.set_notification_cursors(page_cursors);
        app.state.set_notifications_current_cursor_index(target);
        Ok(())
    }

    async fn do_load_notification_settings(
        &mut self,
        subject: atrium_api::types::string::Did,
        handle: String,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let preferences = bsky::notification_preferences(agent.as_ref()).await?;
        let profile = bsky::profile(agent.as_ref(), subject.clone().into()).await?;
        let activity = profile
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.activity_subscription.clone())
            .unwrap_or_else(|| {
                atrium_api::app::bsky::notification::defs::ActivitySubscriptionData {
                    post: false,
                    reply: false,
                }
                .into()
            });
        self.app.lock().await.notification_settings =
            Some(crate::app::notifications::NotificationSettings {
                preferences,
                category: 0,
                activity_subject: Some((subject, handle, activity)),
            });
        Ok(())
    }

    async fn do_toggle_notification_follow(
        &mut self,
        subject: atrium_api::types::string::Did,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let profile = bsky::profile(agent.as_ref(), subject.into()).await?;
        bsky::toggle_follow(agent.as_ref(), &profile).await?;
        self.do_load_notifications(NotificationEvent::Reload).await
    }

    async fn do_like_notification_author(
        &mut self,
        subject: atrium_api::types::string::Did,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let content = bsky::profile_content(
            agent.as_ref(),
            subject.clone().into(),
            crate::app::profile::ProfileSection::Posts,
        )
        .await?;
        let post = match content {
            crate::app::profile::ProfileContent::Posts(posts) => posts
                .into_iter()
                .map(|item| item.post.clone())
                .find(|post| post.author.did == subject),
            _ => None,
        }
        .ok_or_else(|| eyre!("this account has no post to like"))?;
        if post
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.like.as_ref())
            .is_none()
        {
            bsky::like(
                agent.as_ref(),
                self.app.lock().await.state.get_did(),
                post.cid.clone(),
                post.uri.clone(),
            )
            .await?;
        }
        Ok(())
    }

    async fn do_save_notification_preferences(
        &self,
        preferences: atrium_api::app::bsky::notification::defs::Preferences,
    ) -> Result<()> {
        bsky::put_notification_preferences(self.agent().await?.as_ref(), preferences).await
    }

    async fn do_save_activity_subscription(
        &self,
        subject: atrium_api::types::string::Did,
        activity: atrium_api::app::bsky::notification::defs::ActivitySubscription,
    ) -> Result<()> {
        bsky::put_activity_subscription(self.agent().await?.as_ref(), subject, activity).await
    }

    async fn do_search(&mut self, event: SearchEvent) -> Result<()> {
        let (current, cursors, existing_query) = {
            let app = self.app.lock().await;
            (
                app.state.get_search_current_cursor_index(),
                app.state.get_search_cursors(),
                app.state.get_search_query(),
            )
        };
        let target = match &event {
            SearchEvent::Load(_) => 0,
            SearchEvent::Reload => current,
            SearchEvent::Prev if current > 0 => current - 1,
            SearchEvent::Prev => return Ok(()),
            SearchEvent::Next => current + 1,
        };
        let cursor = if matches!(event, SearchEvent::Load(_)) {
            None
        } else {
            cursors.get(target).cloned().flatten()
        };
        if matches!(event, SearchEvent::Next) && cursor.is_none() {
            return Ok(());
        }
        let query = match &event {
            SearchEvent::Load(query) => query.clone(),
            _ => existing_query.ok_or_else(|| eyre!("no search query is active"))?,
        };

        let results =
            bsky::search(self.agent().await?.as_ref(), query.clone(), cursor.clone()).await?;
        let moderation = self.app.lock().await.state.moderation();
        let image_urls = results
            .posts
            .iter()
            .flat_map(|post| bsky::post_image_urls(post, &moderation))
            .collect::<Vec<_>>();
        let page_cursors = updated_cursors(
            if matches!(event, SearchEvent::Load(_)) {
                vec![None]
            } else {
                cursors
            },
            target,
            cursor,
            results.cursor.clone(),
        );

        let mut app = self.app.lock().await;
        app.state.set_search_query(Some(query));
        app.state.set_search_results(Some(
            results.posts.iter().map(|post| post.data.clone()).collect(),
        ));
        app.state.set_search_cursors(page_cursors);
        app.state.set_search_current_cursor_index(target);
        app.state.set_tab(Tab::Search);
        app.state.move_search_scroll_top();
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_like(&mut self) -> Result<()> {
        let (did, feed) = {
            let app = self.app.lock().await;
            (
                app.state.get_did(),
                app.state
                    .get_current_feed()
                    .ok_or_else(|| eyre!("no timeline post is selected"))?,
            )
        };
        bsky::toggle_like(self.agent().await?.as_ref(), did, feed).await?;
        self.do_load_timeline(TimelineEvent::Reload).await
    }

    async fn do_repost(&mut self) -> Result<()> {
        let (did, feed) = {
            let app = self.app.lock().await;
            (
                app.state.get_did(),
                app.state
                    .get_current_feed()
                    .ok_or_else(|| eyre!("no timeline post is selected"))?,
            )
        };
        bsky::toggle_repost(self.agent().await?.as_ref(), did, feed).await?;
        self.do_load_timeline(TimelineEvent::Reload).await
    }

    async fn do_reply(&mut self) -> Result<()> {
        let (feed, text) = {
            let app = self.app.lock().await;
            (
                app.state
                    .get_current_feed()
                    .ok_or_else(|| eyre!("no timeline post is selected"))?,
                app.state.get_input().value().to_string(),
            )
        };
        let parent = strong_ref::MainData {
            cid: feed.post.cid.clone(),
            uri: feed.post.uri.clone(),
        };
        let root = bsky::reply_root(&feed).unwrap_or_else(|| parent.clone());
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(
            self.agent().await?.as_ref(),
            drafts,
            Some(
                ReplyRefData {
                    root: root.into(),
                    parent: parent.into(),
                }
                .into(),
            ),
        )
        .await?;
        self.finish_composer().await;
        self.do_load_timeline(TimelineEvent::Reload).await
    }

    async fn do_search_like(&mut self) -> Result<()> {
        let (did, post) = self.selected_search_post().await?;
        bsky::toggle_like_post_view(self.agent().await?.as_ref(), did, post).await?;
        self.do_search(SearchEvent::Reload).await
    }

    async fn do_search_repost(&mut self) -> Result<()> {
        let (did, post) = self.selected_search_post().await?;
        bsky::toggle_repost_post_view(self.agent().await?.as_ref(), did, post).await?;
        self.do_search(SearchEvent::Reload).await
    }

    async fn selected_search_post(
        &self,
    ) -> Result<(
        atrium_api::types::string::Did,
        atrium_api::app::bsky::feed::defs::PostViewData,
    )> {
        let app = self.app.lock().await;
        Ok((
            app.state.get_did(),
            app.state
                .get_current_search_result()
                .ok_or_else(|| eyre!("no search result is selected"))?,
        ))
    }

    async fn do_search_reply(&mut self) -> Result<()> {
        let (_, post) = self.selected_search_post().await?;
        let text = self.app.lock().await.state.get_input().value().to_string();
        let subject = strong_ref::MainData {
            cid: post.cid.clone(),
            uri: post.uri.clone(),
        };
        let root = bsky::reply_root_for_post(self.agent().await?.as_ref(), &post).await?;
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(
            self.agent().await?.as_ref(),
            drafts,
            Some(
                ReplyRefData {
                    root: root.into(),
                    parent: subject.into(),
                }
                .into(),
            ),
        )
        .await?;
        self.finish_composer().await;
        self.do_search(SearchEvent::Reload).await
    }

    async fn finish_composer(&self) {
        let mut app = self.app.lock().await;
        app.state.set_mode(Mode::Normal);
        app.state.set_input(Input::default());
    }

    async fn handle_feature_event(&mut self, event: FeatureEvent) -> Result<()> {
        use crate::app::feature_panel::{FeaturePromptAction, FeatureSection, SettingKey};

        match event {
            FeatureEvent::Load(section) => self.load_feature_section(section).await,
            FeatureEvent::OpenList(uri) => {
                let did = self.app.lock().await.state.get_did();
                let rows = bsky::feature_services::list_detail(
                    self.agent().await?.as_ref(),
                    uri.clone(),
                    did,
                )
                .await?;
                self.app
                    .lock()
                    .await
                    .set_feature_rows(format!("List · {uri}"), rows, true);
                Ok(())
            }
            FeatureEvent::OpenStarterPack(uri) => {
                let rows = bsky::feature_services::starter_pack_detail(
                    self.agent().await?.as_ref(),
                    uri.clone(),
                )
                .await?;
                self.app
                    .lock()
                    .await
                    .set_feature_rows(format!("Starter Pack · {uri}"), rows, true);
                Ok(())
            }
            FeatureEvent::OpenConversation(convo_id) => self.do_open_conversation(convo_id).await,
            FeatureEvent::OpenLabeler(did) => {
                let rows = bsky::feature_services::labeler_detail(
                    self.agent().await?.as_ref(),
                    did.clone(),
                )
                .await?;
                self.app.lock().await.set_feature_rows(
                    format!("Labeler · {}", did.as_str()),
                    rows,
                    true,
                );
                Ok(())
            }
            FeatureEvent::Submit(action, value) => {
                let agent = self.agent().await?;
                match action {
                    FeaturePromptAction::CreateList => {
                        let fields = crate::app::feature_panel::split_fields(&value, 2)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::create_list(
                            agent.as_ref(),
                            fields[0].clone(),
                            fields[1].clone(),
                            fields.get(2).cloned().filter(|value| !value.is_empty()),
                        )
                        .await?;
                    }
                    FeaturePromptAction::EditList { uri, purpose } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 1)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::edit_list(
                            agent.as_ref(),
                            &uri,
                            purpose,
                            fields[0].clone(),
                            fields.get(1).cloned().filter(|value| !value.is_empty()),
                        )
                        .await?;
                    }
                    FeaturePromptAction::AddListMember { list_uri } => {
                        let actor = value.parse().map_err(eyre::Report::msg)?;
                        let profile = bsky::profile(agent.as_ref(), actor).await?;
                        bsky::feature_services::add_list_member(
                            agent.as_ref(),
                            list_uri,
                            profile.did.clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::CreateStarterPack => {
                        let fields = crate::app::feature_panel::split_fields(&value, 3)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::create_starter_pack(
                            agent.as_ref(),
                            fields[0].clone(),
                            Some(fields[1].clone()).filter(|value| !value.is_empty()),
                            fields[2].clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::EditStarterPack { uri } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 3)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::edit_starter_pack(
                            agent.as_ref(),
                            &uri,
                            fields[0].clone(),
                            Some(fields[1].clone()).filter(|value| !value.is_empty()),
                            fields[2].clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::NewConversation => {
                        let actor = value.parse().map_err(eyre::Report::msg)?;
                        let profile = bsky::profile(agent.as_ref(), actor).await?;
                        let convo_id = bsky::feature_services::start_conversation(
                            agent.as_ref(),
                            profile.did.clone(),
                        )
                        .await?;
                        return self.do_open_conversation(convo_id).await;
                    }
                    FeaturePromptAction::SendMessage { convo_id } => {
                        bsky::feature_services::send_dm(agent.as_ref(), convo_id.clone(), value)
                            .await?;
                        return self.do_open_conversation(convo_id).await;
                    }
                    FeaturePromptAction::AddMutedWord => {
                        bsky::feature_services::add_muted_word(agent.as_ref(), value).await?;
                    }
                    FeaturePromptAction::AddLabeler => {
                        let did = atrium_api::types::string::Did::new(value)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::toggle_labeler(agent.as_ref(), did).await?;
                    }
                    FeaturePromptAction::SetLabelVisibility { labeler, label } => {
                        bsky::feature_services::set_label_visibility(
                            agent.as_ref(),
                            Some(labeler),
                            label,
                            value,
                        )
                        .await?;
                    }
                    FeaturePromptAction::Report { subject } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 1)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::report(
                            agent.as_ref(),
                            subject,
                            &fields[0],
                            fields.get(1).cloned(),
                        )
                        .await?;
                        self.app.lock().await.feature_panel = None;
                        return Ok(());
                    }
                    FeaturePromptAction::AddAccount => {
                        let fields = crate::app::feature_panel::split_fields(&value, 2)
                            .map_err(eyre::Report::msg)?;
                        let mut config = crate::app::config::AppConfig::load()?;
                        if !config
                            .accounts
                            .iter()
                            .any(|account| account.identifier == fields[0])
                        {
                            config.accounts.push(crate::app::config::AccountConfig {
                                identifier: fields[0].clone(),
                                service_url: fields[1].clone(),
                            });
                        }
                        config.save()?;
                    }
                    FeaturePromptAction::EditSetting(setting) => {
                        if setting == SettingKey::IncomingDm {
                            bsky::feature_services::set_incoming_dm(agent.as_ref(), value).await?;
                        } else {
                            let mut config = crate::app::config::AppConfig::load()?;
                            match setting {
                                SettingKey::Images => {
                                    config.ui.show_images = value
                                        .parse()
                                        .map_err(|_| eyre!("images must be true or false"))?
                                }
                                SettingKey::DateFormat => config.ui.date_format = value,
                                SettingKey::Language => config.ui.language = value,
                                SettingKey::AccentColor => config.ui.accent_color = value,
                                SettingKey::Keybindings => {
                                    let fields = crate::app::feature_panel::split_fields(&value, 2)
                                        .map_err(eyre::Report::msg)?;
                                    config
                                        .ui
                                        .keybindings
                                        .insert(fields[0].clone(), fields[1].clone());
                                }
                                SettingKey::IncomingDm => unreachable!(),
                            }
                            config.save()?;
                            self.app.lock().await.set_ui_config(config.ui.clone());
                        }
                    }
                }
                let section = self
                    .app
                    .lock()
                    .await
                    .feature_panel()
                    .map(|panel| panel.section)
                    .unwrap_or(FeatureSection::Lists);
                self.load_feature_section(section).await
            }
            FeatureEvent::DeleteRecord(uri) => {
                bsky::feature_services::delete_record(self.agent().await?.as_ref(), &uri).await?;
                let section = self
                    .app
                    .lock()
                    .await
                    .feature_panel()
                    .map(|p| p.section)
                    .unwrap_or(FeatureSection::Lists);
                self.load_feature_section(section).await
            }
            FeatureEvent::ToggleModerationList { uri, muted } => {
                bsky::feature_services::toggle_moderation_list(
                    self.agent().await?.as_ref(),
                    uri,
                    muted,
                )
                .await?;
                self.load_feature_section(FeatureSection::Lists).await
            }
            FeatureEvent::ToggleConversationMute { convo_id, muted } => {
                bsky::feature_services::toggle_conversation_mute(
                    self.agent().await?.as_ref(),
                    convo_id,
                    muted,
                )
                .await?;
                self.load_feature_section(FeatureSection::DirectMessages)
                    .await
            }
            FeatureEvent::RemoveMutedWord(word) => {
                bsky::feature_services::remove_muted_word(self.agent().await?.as_ref(), &word)
                    .await?;
                self.load_feature_section(FeatureSection::Moderation).await
            }
            FeatureEvent::ToggleLabeler(did) => {
                bsky::feature_services::toggle_labeler(self.agent().await?.as_ref(), did).await?;
                self.load_feature_section(FeatureSection::Moderation).await
            }
            FeatureEvent::UseListFeed { uri, name } => {
                self.app
                    .lock()
                    .await
                    .state
                    .activate_feed(crate::app::feed::FeedDescriptor::list(uri, name));
                self.do_load_timeline(TimelineEvent::Load).await
            }
            FeatureEvent::SaveList(uri) => {
                let mut config = crate::app::config::AppConfig::load()?;
                if config.saved_lists.iter().any(|saved| saved == &uri) {
                    config.saved_lists.retain(|saved| saved != &uri);
                } else {
                    config.saved_lists.push(uri);
                }
                config.save()?;
                self.load_feature_section(FeatureSection::Lists).await
            }
            FeatureEvent::JoinStarterPack(actors) => {
                let agent = self.agent().await?;
                for actor in actors {
                    let profile = bsky::profile(agent.as_ref(), actor).await?;
                    if profile
                        .viewer
                        .as_ref()
                        .and_then(|viewer| viewer.following.as_ref())
                        .is_none()
                    {
                        bsky::toggle_follow(agent.as_ref(), &profile).await?;
                    }
                }
                Ok(())
            }
            FeatureEvent::SwitchAccount(identifier) => {
                let mut config = crate::app::config::AppConfig::load()?;
                config.active_account = Some(identifier);
                config.save()?;
                self.app.lock().await.feature_panel = None;
                self.do_initialize().await
            }
            FeatureEvent::ToggleThreadMute { root, muted } => {
                bsky::feature_services::toggle_thread_mute(
                    self.agent().await?.as_ref(),
                    root.clone(),
                    muted,
                )
                .await?;
                let mut config = crate::app::config::AppConfig::load()?;
                if muted {
                    config.muted_threads.retain(|uri| uri != &root);
                } else if !config.muted_threads.contains(&root) {
                    config.muted_threads.push(root);
                }
                config.save()?;
                if self.app.lock().await.feature_panel().is_some() {
                    self.load_feature_section(crate::app::feature_panel::FeatureSection::Moderation)
                        .await
                } else {
                    Ok(())
                }
            }
            FeatureEvent::ToggleHiddenReply { root, reply } => {
                let uri = root.uri.clone();
                bsky::feature_services::toggle_hidden_reply(
                    self.agent().await?.as_ref(),
                    &root,
                    reply,
                )
                .await?;
                self.do_load_thread(uri).await
            }
            FeatureEvent::DetachQuote { post, quote } => {
                bsky::feature_services::detach_quote(
                    self.agent().await?.as_ref(),
                    post,
                    quote.clone(),
                )
                .await?;
                self.do_load_thread(quote).await
            }
        }
    }

    async fn load_feature_section(
        &mut self,
        section: crate::app::feature_panel::FeatureSection,
    ) -> Result<()> {
        use crate::app::feature_panel::{FeatureRow, FeatureSection, FeatureTarget, SettingKey};
        let did = self.app.lock().await.state.get_did();
        let rows = match section {
            FeatureSection::Lists => {
                let agent = self.agent().await?;
                let mut rows =
                    bsky::feature_services::own_lists(agent.as_ref(), did.clone()).await?;
                let config = crate::app::config::AppConfig::load()?;
                for uri in config.saved_lists {
                    if !rows.iter().any(|row| matches!(&row.target, FeatureTarget::List { uri: existing, .. } if existing == &uri)) {
                        if let Ok(row) = bsky::feature_services::list_overview(agent.as_ref(), uri, &did).await {
                            rows.push(row);
                        }
                    }
                }
                rows
            }
            FeatureSection::StarterPacks => {
                bsky::feature_services::starter_packs(self.agent().await?.as_ref(), did).await?
            }
            FeatureSection::Discovery => {
                bsky::feature_services::discovery(self.agent().await?.as_ref(), did).await?
            }
            FeatureSection::DirectMessages => {
                bsky::feature_services::conversations(self.agent().await?.as_ref(), &did).await?
            }
            FeatureSection::Moderation => {
                let mut rows = bsky::feature_services::moderation_preferences_rows(
                    self.agent().await?.as_ref(),
                )
                .await?;
                let config = crate::app::config::AppConfig::load()?;
                rows.extend(config.muted_threads.into_iter().map(|uri| FeatureRow {
                    title: "Muted thread".into(),
                    detail: uri.clone(),
                    target: FeatureTarget::MutedThread(uri),
                    unread: false,
                }));
                rows
            }
            FeatureSection::Settings => {
                let config = crate::app::config::AppConfig::load()?;
                let mut rows = config
                    .all_accounts()
                    .into_iter()
                    .map(|account| FeatureRow {
                        title: format!(
                            "{}{}",
                            if config.active_account().identifier == account.identifier {
                                "● "
                            } else {
                                "  "
                            },
                            account.identifier
                        ),
                        detail: account.service_url,
                        target: FeatureTarget::Account(account.identifier),
                        unread: false,
                    })
                    .collect::<Vec<_>>();
                rows.extend([
                    setting_row(
                        "Images",
                        config.ui.show_images.to_string(),
                        SettingKey::Images,
                    ),
                    setting_row("Date format", config.ui.date_format, SettingKey::DateFormat),
                    setting_row("Language", config.ui.language, SettingKey::Language),
                    setting_row(
                        "Accent color",
                        config.ui.accent_color,
                        SettingKey::AccentColor,
                    ),
                    setting_row("Keybinding", "action | key".into(), SettingKey::Keybindings),
                    setting_row(
                        "Incoming DMs",
                        "all / following / none".into(),
                        SettingKey::IncomingDm,
                    ),
                ]);
                rows
            }
        };
        self.app
            .lock()
            .await
            .set_feature_rows(section.title().to_owned(), rows, false);
        Ok(())
    }

    async fn do_open_conversation(&mut self, convo_id: String) -> Result<()> {
        let rows =
            bsky::feature_services::conversation(self.agent().await?.as_ref(), convo_id.clone())
                .await?;
        let child = self
            .app
            .lock()
            .await
            .feature_panel()
            .is_some_and(|panel| !panel.title.starts_with("Conversation ·"));
        self.app
            .lock()
            .await
            .set_feature_rows(format!("Conversation · {convo_id}"), rows, child);
        Ok(())
    }
}

fn setting_row(
    title: &str,
    value: String,
    key: crate::app::feature_panel::SettingKey,
) -> crate::app::feature_panel::FeatureRow {
    crate::app::feature_panel::FeatureRow {
        title: title.to_owned(),
        detail: value,
        target: crate::app::feature_panel::FeatureTarget::Setting(key),
        unread: false,
    }
}

fn drafts_with_default_language(text: &str) -> Result<Vec<crate::app::composer::PostDraft>> {
    let mut drafts = crate::app::composer::parse_drafts(text)?;
    let language = crate::app::config::AppConfig::load()?.ui.language;
    if language != "auto" && !language.trim().is_empty() {
        let language =
            atrium_api::types::string::Language::new(language).map_err(eyre::Report::msg)?;
        for draft in &mut drafts {
            if draft.langs.is_none() {
                draft.langs = Some(vec![language.clone()]);
            }
        }
    }
    Ok(drafts)
}

fn updated_cursors(
    mut cursors: Vec<Option<String>>,
    target: usize,
    current: Option<String>,
    next: Option<String>,
) -> Vec<Option<String>> {
    cursors.resize(target + 1, None);
    cursors[target] = current;
    cursors.truncate(target + 1);
    cursors.push(next);
    cursors
}

fn count_new_posts(
    old: &[atrium_api::app::bsky::feed::defs::FeedViewPost],
    new: &[atrium_api::app::bsky::feed::defs::FeedViewPost],
) -> usize {
    let Some(first_old_uri) = old.first().map(|feed| feed.post.uri.as_str()) else {
        return 0;
    };
    new.iter()
        .take_while(|feed| feed.post.uri != first_old_uri)
        .count()
}

fn operation_name(event: &IoEvent) -> &'static str {
    match event {
        IoEvent::Initialize => "Initialization",
        IoEvent::LoadTimeline(_) => "Timeline request",
        IoEvent::LoadNotifications(_) => "Notification request",
        IoEvent::LoadNotificationSettings(_, _) => "Notification settings request",
        IoEvent::SaveNotificationPreferences(_) => "Notification settings update",
        IoEvent::SaveActivitySubscription { .. } => "Activity notification update",
        IoEvent::ToggleNotificationFollow(_) => "Follow update",
        IoEvent::LikeNotificationAuthor(_) => "Like back",
        IoEvent::SendPost => "Post",
        IoEvent::Like | IoEvent::SearchLike => "Like",
        IoEvent::Repost | IoEvent::SearchRepost => "Repost",
        IoEvent::Reply | IoEvent::SearchReply => "Reply",
        IoEvent::LoadThread(_) => "Thread request",
        IoEvent::LoadInteractions(_, _, _) => "Post interactions request",
        IoEvent::SearchUsers(_) => "User search",
        IoEvent::LoadProfile(_) | IoEvent::LoadProfileSection(_) => "Profile request",
        IoEvent::ToggleFollow => "Follow request",
        IoEvent::LoadConnections(_, _) => "Social graph request",
        IoEvent::Moderate(_) => "Moderation request",
        IoEvent::LoadFeedCatalog
        | IoEvent::SearchFeeds(_)
        | IoEvent::SelectFeed(_)
        | IoEvent::ToggleSavedFeed(_) => "Feed request",
        IoEvent::DeletePost(_) => "Delete post",
        IoEvent::PreviewLink(_) => "Link preview",
        IoEvent::Search(_) => "Search",
        IoEvent::Feature(_) => "Extended client request",
    }
}

fn user_error(operation: &str, error: &eyre::Report) -> String {
    let detail = format!("{error:#}");
    let lower = detail.to_ascii_lowercase();
    let category = if lower.contains("429") || lower.contains("rate limit") {
        "Rate limit exceeded"
    } else if lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("authentication")
        || lower.contains("invalid identifier or password")
    {
        "Authentication failed"
    } else if lower.contains("timeout")
        || lower.contains("connect")
        || lower.contains("dns")
        || lower.contains("network")
    {
        "Network request failed"
    } else {
        "Operation failed"
    };
    format!("{operation}: {category}. {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_history_is_replaced_after_reload() {
        let cursors = vec![None, Some("page-2".into()), Some("stale".into())];
        let updated = updated_cursors(cursors, 1, Some("page-2".into()), Some("fresh".into()));
        assert_eq!(
            updated,
            vec![None, Some("page-2".into()), Some("fresh".into())]
        );
    }
}
