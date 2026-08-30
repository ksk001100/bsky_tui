pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;
pub mod logging;
pub mod utils;

use std::{io::stdout, time::Duration};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;

use crate::{
    app::{command::Command, message::Message, ui, App, AppReturn, Update},
    inputs::{events::Events, InputEvent},
    io::handler::EffectEnvelope,
};

pub async fn start_ui(
    mut app: App,
    command_tx: tokio::sync::mpsc::Sender<Command>,
    effect_rx: tokio::sync::mpsc::Receiver<EffectEnvelope>,
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
    app.configure_images(picker);

    let result = run_ui(
        &mut terminal,
        &mut app,
        &command_tx,
        effect_rx,
        skip_splash,
        splash,
    )
    .await;

    let _ = terminal.clear();
    let _ = terminal.show_cursor();
    session.restore_with_backend(terminal.backend_mut());

    result
}

async fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    command_tx: &tokio::sync::mpsc::Sender<Command>,
    mut effect_rx: tokio::sync::mpsc::Receiver<EffectEnvelope>,
    skip_splash: bool,
    splash: String,
) -> Result<()> {
    let tick_rate = Duration::from_millis(200);
    let mut events = Events::new(tick_rate);

    let update = app.init();
    execute_commands(app, command_tx, update.commands).await?;

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
                let initialized = app.state.get_timeline().is_some() || app.error().is_some();
                if initialized {
                    break;
                }
                tokio::select! {
                    envelope = effect_rx.recv() => {
                        let envelope = envelope.ok_or_else(|| eyre::eyre!("effect runtime is unavailable"))?;
                        let update = apply_effect(app, envelope);
                        execute_commands(app, command_tx, update.commands).await?;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }

            if app.error().is_some() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
            split_splash.pop();
        }
    }

    loop {
        terminal.draw(|rect| ui::render::<CrosstermBackend<std::io::Stdout>>(rect, app))?;

        let update = tokio::select! {
            input = events.next() => {
                match input {
                    InputEvent::Input(key) => app.update(Message::KeyPressed(key)),
                    InputEvent::Mouse(mouse) => app.update(Message::Mouse(mouse)),
                    InputEvent::Resize(_, _) => Update {
                        control: AppReturn::Continue,
                        commands: Vec::new(),
                    },
                    InputEvent::Tick => app.update(Message::Tick),
                }
            }
            envelope = effect_rx.recv() => {
                let envelope = envelope.ok_or_else(|| eyre::eyre!("effect runtime is unavailable"))?;
                apply_effect(app, envelope)
            }
        };
        execute_commands(app, command_tx, update.commands).await?;

        if update.control == AppReturn::Exit {
            events.close();
            break;
        }
    }

    Ok(())
}

fn apply_effect(app: &mut App, envelope: EffectEnvelope) -> Update {
    let update = app.update(Message::effect(envelope.message));
    let context = envelope
        .context_event
        .as_ref()
        .map(|event| app.effect_context(event));
    let _ = envelope.applied.send(context);
    update
}

async fn execute_commands(
    app: &mut App,
    command_tx: &tokio::sync::mpsc::Sender<Command>,
    commands: Vec<Command>,
) -> Result<()> {
    for command in commands {
        match command {
            Command::LoadImages(urls) => app.queue_images(urls),
            Command::PollImages => app.poll_images(),
            Command::OpenUrl { url, error_context } => {
                if let Err(error) = webbrowser::open(&url) {
                    let message = format!("{error_context}: {error}");
                    let update = app.update(Message::effect(
                        crate::app::message::EffectMessage::RuntimeError(message),
                    ));
                    debug_assert!(update.commands.is_empty());
                }
            }
            Command::CopyToClipboard { value, label } => {
                if let Err(error) = crate::app::copy_osc52(&value) {
                    let message = format!("Could not copy {label}: {error}");
                    let update = app.update(Message::effect(
                        crate::app::message::EffectMessage::RuntimeError(message),
                    ));
                    debug_assert!(update.commands.is_empty());
                }
            }
            command @ Command::Io { .. } => command_tx
                .send(command)
                .await
                .map_err(|_| eyre::eyre!("background worker is unavailable"))?,
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
