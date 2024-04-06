use std::sync::Arc;

use atrium_api::{app::bsky::feed::post::ReplyRef, com::atproto::repo::strong_ref};
use eyre::Result;
use tui_input::Input;

use super::IoEvent;
use crate::{
    app::{config::AppConfig, state::Mode, App},
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
        let _ = match io_event {
            IoEvent::Initialize => self.do_initialize().await,
            IoEvent::LoadTimeline => self.do_load_timeline().await,
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
            let config = AppConfig::load()?;
            let mut app = self.app.lock().await;
            let agent =
                bsky::agent_with_session(config.email.clone(), config.password.clone()).await?;
            let session =
                bsky::session(&agent, config.email.clone(), config.password.clone()).await?;
            app.initialized(agent, session.handle.to_string(), session.did.to_string(), config);
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_load_timeline(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let cursor = {
            let app = self.app.lock().await;
            app.state.get_cursor()
        };
        {
            // let timeline = bsky::timeline(&agent, cursor).await?;
            let timeline = bsky::timeline(&agent, None).await?;
            let mut app = self.app.lock().await;
            app.state.set_timeline(Some(timeline.feed));
            app.state.set_cursor(timeline.cursor);
        }

        Ok(())
    }

    async fn do_send_post(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let did = {
            let app = self.app.lock().await;
            app.state.get_did()
        };
        let text = {
            let app = self.app.lock().await;
            app.state.get_input().value().to_string()
        };
        {
            let mut app = self.app.lock().await;
            app.state.set_mode(Mode::Normal);
            app.state.set_input(Input::default());
        }
        bsky::send_post(&agent, did, text, None).await?;
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_load_notifications(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let notifications = bsky::notifications(&agent).await?;
        let mut app = self.app.lock().await;
        app.state
            .set_notifications(Some(notifications.notifications));

        Ok(())
    }

    async fn do_like(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let did = {
            let app = self.app.lock().await;
            app.state.get_did()
        };
        let current_feed = {
            let app = self.app.lock().await;
            app.state.get_current_feed().unwrap()
        };

        bsky::toggle_like(&agent, did, current_feed).await?;
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_repost(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let did = {
            let app = self.app.lock().await;
            app.state.get_did()
        };
        let current_feed = {
            let app = self.app.lock().await;
            app.state.get_current_feed().unwrap()
        };

        bsky::toggle_repost(&agent, did, current_feed).await?;
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_reply(&mut self) -> Result<()> {
        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let did = {
            let app = self.app.lock().await;
            app.state.get_did()
        };
        let current_feed = {
            let app = self.app.lock().await;
            app.state.get_current_feed().unwrap()
        };
        let text = {
            let app = self.app.lock().await;
            app.state.get_input().value().to_string()
        };
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

        {
            let mut app = self.app.lock().await;
            app.state.set_mode(Mode::Normal);
            app.state.set_input(Input::default());
        }

        bsky::send_post(&agent, did, text, Some(reply)).await?;
        self.do_load_timeline().await?;

        Ok(())
    }
}
