pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;

use crate::app::ui;
use app::{App, AppReturn};
use eyre::Result;
use inputs::events::Events;
use inputs::InputEvent;
use io::IoEvent;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;

pub async fn start_ui(app: &Arc<tokio::sync::Mutex<App>>) -> Result<()> {
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    // terminal.hide_cursor()?;
    terminal.show_cursor()?;

    let tick_rate = Duration::from_millis(200);
    let mut events = Events::new(tick_rate);

    {
        let mut app = app.lock().await;
        app.dispatch(IoEvent::Initialize).await;
    }

    loop {
        let mut app = app.lock().await;

        terminal.draw(|rect| ui::render(rect, &app))?;

        let result = match events.next().await {
            InputEvent::Input(key) => app.do_action(key).await,
            InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            events.close();
            break;
        }
    }

    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
