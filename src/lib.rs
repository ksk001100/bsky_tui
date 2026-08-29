pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;
pub mod utils;

use std::{io::stdout, sync::Arc, time::Duration};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;

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
    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    app.lock().await.configure_images(picker);

    let result = run_ui(&mut terminal, app, skip_splash, splash).await;

    let _ = terminal.clear();
    let _ = terminal.show_cursor();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = crossterm::terminal::disable_raw_mode();

    result
}

async fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &Arc<tokio::sync::Mutex<App>>,
    skip_splash: bool,
    splash: String,
) -> Result<()> {
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
                if app.state.get_timeline().is_some() || app.error().is_some() {
                    break;
                }
                drop(app);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            if app.lock().await.error().is_some() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
            split_splash.pop();
        }
    }

    loop {
        let mut app = app.lock().await;

        terminal.draw(|rect| ui::render::<CrosstermBackend<std::io::Stdout>>(rect, &mut app))?;

        let result = match events.next().await {
            InputEvent::Input(key) => app.do_action(key).await,
            InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            events.close();
            break;
        }
    }

    Ok(())
}
