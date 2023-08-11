use super::IoEvent;
use crate::app::state::Mode;
use crate::app::App;
use crate::bsky;

use eyre::Result;
use std::sync::Arc;

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
        };

        let mut app = self.app.lock().await;
        app.loaded();
    }

    async fn do_initialize(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            let agent = bsky::agent_with_session().await?;
            app.initialized(agent);
        }
        self.do_load_timeline().await?;

        Ok(())
    }

    async fn do_load_timeline(&mut self) -> Result<()> {
        let mut app = self.app.lock().await;
        let agent = app.state.get_agent().unwrap();
        let timeline = bsky::timeline(&agent).await?;
        app.state.set_feed(timeline.feed);

        Ok(())
    }

    async fn do_send_post(&mut self) -> Result<()> {
        {
            let mut app = self.app.lock().await;
            let agent = app.state.get_agent().unwrap();
            bsky::send_post(&agent, app.state.get_input_text().unwrap_or("".to_string())).await?;
            app.state.set_mode(Mode::Normal);
            app.state.set_input_text("".to_string());
        }
        self.do_load_timeline().await?;

        Ok(())
    }
}
