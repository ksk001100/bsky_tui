use std::sync::Arc;

use atrium_api::{app::bsky::feed::post::ReplyRefData, com::atproto::repo::strong_ref};
use bsky_sdk::BskyAgent;
use eyre::{eyre, Result};
use tui_input::Input;

use super::{IoEvent, NotificationEvent, SearchEvent, TimelineEvent};
use crate::{
    app::{config::AppConfig, state::Mode, state::Tab, App},
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
            IoEvent::Like => self.do_like().await,
            IoEvent::Repost => self.do_repost().await,
            IoEvent::Reply => self.do_reply().await,
            IoEvent::Search(action) => self.do_search(action).await,
            IoEvent::SearchLike => self.do_search_like().await,
            IoEvent::SearchRepost => self.do_search_repost().await,
            IoEvent::SearchReply => self.do_search_reply().await,
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
        let (agent, session) = bsky::agent_with_session(
            config.service_url.clone(),
            config.email.clone(),
            config.password.clone(),
        )
        .await?;
        {
            let mut app = self.app.lock().await;
            app.initialized(agent, session.handle.clone(), session.did.clone(), config);
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

        let timeline = bsky::timeline(self.agent().await?.as_ref(), cursor.clone()).await?;
        let image_urls = timeline
            .feed
            .iter()
            .flat_map(|feed| bsky::post_image_urls(&feed.post))
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
        app.state.set_timeline(Some(timeline.feed.clone()));
        app.state.set_cursors(page_cursors);
        app.state.set_tl_current_cursor_index(target);
        app.state.move_tl_scroll_top();
        app.queue_images(image_urls);
        Ok(())
    }

    async fn do_send_post(&mut self) -> Result<()> {
        let (did, text) = {
            let app = self.app.lock().await;
            (
                app.state.get_did(),
                app.state.get_input().value().to_string(),
            )
        };
        bsky::validate_post_text(&text)?;
        bsky::send_post(self.agent().await?.as_ref(), did, text, None).await?;
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
        let image_urls = results
            .posts
            .iter()
            .flat_map(|post| bsky::post_image_urls(post))
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
        let (did, feed, text) = {
            let app = self.app.lock().await;
            (
                app.state.get_did(),
                app.state
                    .get_current_feed()
                    .ok_or_else(|| eyre!("no timeline post is selected"))?,
                app.state.get_input().value().to_string(),
            )
        };
        bsky::validate_post_text(&text)?;
        let parent = strong_ref::MainData {
            cid: feed.post.cid.clone(),
            uri: feed.post.uri.clone(),
        };
        let root = bsky::reply_root(&feed).unwrap_or_else(|| parent.clone());
        bsky::send_post(
            self.agent().await?.as_ref(),
            did,
            text,
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
        let (did, post) = self.selected_search_post().await?;
        let text = self.app.lock().await.state.get_input().value().to_string();
        bsky::validate_post_text(&text)?;
        let subject = strong_ref::MainData {
            cid: post.cid.clone(),
            uri: post.uri.clone(),
        };
        let root = bsky::reply_root_for_post(self.agent().await?.as_ref(), &post).await?;
        bsky::send_post(
            self.agent().await?.as_ref(),
            did,
            text,
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

fn operation_name(event: &IoEvent) -> &'static str {
    match event {
        IoEvent::Initialize => "Initialization",
        IoEvent::LoadTimeline(_) => "Timeline request",
        IoEvent::LoadNotifications(_) => "Notification request",
        IoEvent::SendPost => "Post",
        IoEvent::Like | IoEvent::SearchLike => "Like",
        IoEvent::Repost | IoEvent::SearchRepost => "Repost",
        IoEvent::Reply | IoEvent::SearchReply => "Reply",
        IoEvent::Search(_) => "Search",
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
