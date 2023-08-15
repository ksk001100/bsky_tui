use std::sync::Arc;

use atrium_api::{app::bsky::feed::post::ReplyRef, com::atproto::repo::strong_ref};
use eyre::Result;

use super::IoEvent;
use crate::app::state::Mode;
use crate::app::App;
use crate::bsky;

pub struct IoAsyncHandler {
    app: Arc<tokio::sync::Mutex<App>>,
}

impl IoAsyncHandler {
    pub fn new(app: Arc<tokio::sync::Mutex<App>>) -> Self {
        Self { app }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        let _ = match io_event {
            IoEvent::Initialize => self.do_initialize().await,
            IoEvent::LoadFeed => self.do_load_timeline().await,
            IoEvent::SendPost => self.do_send_post().await,
            IoEvent::LoadNotifications => self.do_load_notifications().await,
            IoEvent::Like => self.do_like().await,
            IoEvent::Repost => self.do_repost().await,
            IoEvent::Reply => self.do_reply().await,
        };

        let mut app = self.app.lock().await;
        app.loaded();
    }

    async fn do_initialize(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            let agent = bsky::agent_with_session().await?;
            let session = bsky::session(&agent).await?;
            app.initialized(agent, session.handle);
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_load_timeline(&mut self) -> Result<()> {
        let mut app = self.app.lock().await;
        let agent = app.state.get_agent().unwrap();
        let timeline = bsky::timeline(&agent).await?;
        app.state.set_feeds(timeline.feed);

        Ok(())
    }

    async fn do_send_post(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            let agent = app.state.get_agent().unwrap();
            bsky::send_post(
                &agent,
                app.state.get_input_text().unwrap_or("".to_string()),
                None,
            )
            .await?;
            app.state.set_mode(Mode::Normal);
            app.state.set_input_text("".to_string());
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_load_notifications(&mut self) -> Result<()> {
        let mut app = self.app.lock().await;
        let agent = app.state.get_agent().unwrap();
        let notifications = bsky::notifications(&agent).await?;
        app.state.set_notifications(notifications.notifications);

        Ok(())
    }

    async fn do_like(&mut self) -> Result<()> {
        {
            let app = self.app.lock().await;
            let agent = app.state.get_agent().unwrap();
            let current_feed = app.state.get_current_feed().unwrap();
            bsky::toggle_like(&agent, current_feed).await?;
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_repost(&mut self) -> Result<()> {
        {
            let app = self.app.lock().await;
            let agent = app.state.get_agent().unwrap();
            let current_feed = app.state.get_current_feed().unwrap();
            bsky::toggle_repost(&agent, current_feed).await?;
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_reply(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            let agent = app.state.get_agent().unwrap();
            let current_feed = app.state.get_current_feed().unwrap();
            let reply = ReplyRef {
                root: strong_ref::Main {
                    cid: current_feed.post.cid.clone(),
                    uri: current_feed.post.uri.clone(),
                },
                parent: strong_ref::Main {
                    cid: current_feed.post.cid.clone(),
                    uri: current_feed.post.uri.clone(),
                },
            };
            bsky::send_post(
                &agent,
                app.state.get_input_text().unwrap_or("".to_string()),
                Some(reply),
            )
            .await?;
            app.state.set_mode(Mode::Normal);
            app.state.set_input_text("".to_string());
        }
        self.do_load_timeline().await?;

        Ok(())
    }
}
