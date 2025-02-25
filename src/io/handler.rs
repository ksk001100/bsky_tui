use std::sync::Arc;

use atrium_api::app::bsky::feed::post::ReplyRefData;
use atrium_api::com::atproto::repo::strong_ref;
use eyre::Result;
use tui_input::Input;

use super::{IoEvent, TimelineEvent};
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
            IoEvent::LoadTimeline(action) => self.do_load_timeline(action).await,
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
            app.initialized(agent, session.handle.clone(), session.did.clone(), config);
        }
        self.do_load_timeline(TimelineEvent::Load).await?;

        Ok(())
    }

    async fn do_load_timeline(&mut self, event: TimelineEvent) -> Result<()> {
        let current_cursor_index = {
            let app = self.app.lock().await;
            app.state.get_tl_current_cursor_index()
        };

        if current_cursor_index == 0 && event == TimelineEvent::Prev {
            return Ok(());
        }

        {
            let mut app = self.app.lock().await;
            app.state.set_loading(true);
        }

        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let cursor = match event {
            TimelineEvent::Load => None,
            TimelineEvent::Next => {
                let app = self.app.lock().await;
                app.state.get_next_cursor()
            }
            TimelineEvent::Prev => {
                let app = self.app.lock().await;
                app.state.get_prev_cursor()
            }
            TimelineEvent::Reload => {
                let app = self.app.lock().await;
                app.state.get_current_cursor()
            }
        };
        let current_cursor_index = {
            let app = self.app.lock().await;
            app.state.get_tl_current_cursor_index()
        };

        if event == TimelineEvent::Prev && current_cursor_index == 0 {
            return Ok(());
        }

        {
            let timeline = bsky::timeline(&agent, cursor).await?;
            let mut app = self.app.lock().await;
            app.state.set_timeline(Some(timeline.feed.clone()));

            match event {
                TimelineEvent::Load => {
                    let mut cursors = app.state.get_cursors().clone();
                    cursors.push(timeline.cursor.clone());
                    app.state.set_cursors(cursors);
                }
                TimelineEvent::Next => {
                    let mut cursors = app.state.get_cursors().clone();
                    cursors.push(timeline.cursor.clone());
                    app.state.set_cursors(cursors);
                    app.state
                        .set_tl_current_cursor_index(current_cursor_index + 1);
                }
                TimelineEvent::Prev => {
                    if current_cursor_index == 0 {
                        return Ok(());
                    }
                    app.state
                        .set_tl_current_cursor_index(current_cursor_index - 1);
                }
                _ => (),
            }

            app.state.move_tl_scroll_top();
        }

        {
            let mut app = self.app.lock().await;
            app.state.set_loading(false);
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
        self.do_load_timeline(TimelineEvent::Load).await?;

        Ok(())
    }

    async fn do_load_notifications(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            app.state.set_loading(true);
        }

        let agent = {
            let app = self.app.lock().await;
            app.state.get_agent().unwrap()
        };
        let notifications = bsky::notifications(&agent).await?;
        let mut app = self.app.lock().await;
        app.state
            .set_notifications(Some(notifications.notifications.clone()));
        app.state.set_loading(false);

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
        self.do_load_timeline(TimelineEvent::Reload).await?;

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
        self.do_load_timeline(TimelineEvent::Reload).await?;

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
        let reply = ReplyRefData {
            root: strong_ref::MainData {
                cid: current_feed.post.cid.clone(),
                uri: current_feed.post.uri.clone(),
            }
            .into(),
            parent: strong_ref::MainData {
                cid: current_feed.post.cid.clone(),
                uri: current_feed.post.uri.clone(),
            }
            .into(),
        };

        {
            let mut app = self.app.lock().await;
            app.state.set_mode(Mode::Normal);
            app.state.set_input(Input::default());
        }

        bsky::send_post(&agent, did, text, Some(reply.into())).await?;
        self.do_load_timeline(TimelineEvent::Load).await?;

        Ok(())
    }
}
