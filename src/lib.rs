pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;
pub mod utils;

use std::{io::stdout, sync::Arc, time::Duration};

use eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::{ui, App, AppReturn},
    inputs::{events::Events, InputEvent},
    io::IoEvent,
};

pub async fn start_ui(
    app: &Arc<tokio::sync::Mutex<App>>,
    skip_splash: bool,
    splash: String,
) -> Result<()> {
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    // terminal.show_cursor()?;

    let tick_rate = Duration::from_millis(200);
    let mut events = Events::new(tick_rate);

    {
        let mut app = app.lock().await;
        app.dispatch(IoEvent::Initialize).await;
    }

    if !skip_splash {
        let mut split_splash: Vec<String> = splash.split('\n').map(|s| s.to_string()).collect();
        while !split_splash.is_empty() {
            terminal.draw(|rect| {
                ui::render_splash::<CrosstermBackend<std::io::Stdout>>(
                    rect,
                    split_splash.join("\n"),
                )
            })?;

            loop {
                let app = app.lock().await;
                if app.state.get_timeline().is_some() {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
            split_splash.pop();
        }
    }

    loop {
        let mut app = app.lock().await;

        terminal.draw(|rect| ui::render::<CrosstermBackend<std::io::Stdout>>(rect, &app))?;

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
