pub mod state;
pub mod ui;

use self::state::AppState;
use crate::bsky;
use crate::inputs::key::Key;
use crate::io::IoEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    io_tx: tokio::sync::mpsc::Sender<IoEvent>,
    is_loading: bool,
    pub state: AppState,
}

impl App {
    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let is_loading = false;
        let state = AppState::default();

        Self {
            io_tx,
            is_loading,
            state,
        }
    }

    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        match self.state.get_mode() {
            state::Mode::Normal => match key {
                Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
                Key::Char('r') => {
                    self.dispatch(IoEvent::LoadFeed).await;
                    AppReturn::Continue
                }
                Key::Char('n') => {
                    self.state.set_mode(state::Mode::Post);
                    AppReturn::Continue
                }
                Key::Char('?') => {
                    self.state.set_mode(state::Mode::Help);
                    AppReturn::Continue
                }
                Key::Down | Key::Char('j') => {
                    self.state.move_tl_scroll_down();
                    AppReturn::Continue
                }
                Key::Up | Key::Char('k') => {
                    self.state.move_tl_scroll_up();
                    AppReturn::Continue
                }
                _ => AppReturn::Continue,
            },
            state::Mode::Post => match key {
                Key::Esc => {
                    self.state.set_mode(state::Mode::Normal);
                    AppReturn::Continue
                }
                Key::Enter => {
                    self.dispatch(IoEvent::SendPost).await;
                    AppReturn::Continue
                }
                Key::Left | Key::Ctrl('b') => {
                    self.state.move_input_cursor_left();
                    AppReturn::Continue
                }
                Key::Right | Key::Ctrl('f') => {
                    self.state.move_input_cursor_right();
                    AppReturn::Continue
                }
                Key::Char(c) => {
                    self.state.insert_input_text(c);
                    AppReturn::Continue
                }
                Key::Backspace | Key::Ctrl('h') => {
                    self.state.remove_input_text();
                    AppReturn::Continue
                }
                _ => AppReturn::Continue,
            },
            state::Mode::Help => match key {
                Key::Char('q') | Key::Esc | Key::Char('?') => {
                    self.state.set_mode(state::Mode::Normal);
                    AppReturn::Continue
                }
                _ => AppReturn::Continue,
            },
        }
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        AppReturn::Continue
    }

    pub async fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;
        if self.io_tx.send(action).await.is_err() {
            self.is_loading = false;
        };
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn initialized(&mut self, agent: bsky::Agent) {
        self.state = AppState::initialized(agent);
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }
}
