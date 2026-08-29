pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;
pub mod logging;
pub mod utils;

use std::{io::stdout, sync::Arc, time::Duration};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
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
    let mut session = TerminalSession::enter()?;
    let stdout = session.take_stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    app.lock().await.configure_images(picker);

    let result = run_ui(&mut terminal, app, skip_splash, splash).await;

    let _ = terminal.clear();
    let _ = terminal.show_cursor();
    session.restore_with_backend(terminal.backend_mut());

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
            InputEvent::Mouse(mouse) => app.do_mouse_action(mouse).await,
            InputEvent::Resize(_, _) => AppReturn::Continue,
            InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            events.close();
            break;
        }
    }

    Ok(())
}

struct TerminalSession {
    stdout: Option<std::io::Stdout>,
    restored: bool,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            let _ = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen);
            let _ = crossterm::terminal::disable_raw_mode();
            return Err(error.into());
        }
        Ok(Self {
            stdout: Some(stdout),
            restored: false,
        })
    }

    fn take_stdout(&mut self) -> std::io::Stdout {
        self.stdout.take().unwrap_or_else(stdout)
    }

    fn restore_with_backend(&mut self, backend: &mut CrosstermBackend<std::io::Stdout>) {
        if !self.restored {
            let _ = execute!(backend, DisableMouseCapture, LeaveAlternateScreen);
            let _ = crossterm::terminal::disable_raw_mode();
            self.restored = true;
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.restored {
            let mut stdout = self.stdout.take().unwrap_or_else(stdout);
            let _ = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen);
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}
