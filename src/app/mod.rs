pub mod config;
pub mod state;
pub mod ui;

use self::state::AppState;
use crate::{
    app::{config::AppConfig, state::Tab},
    bsky,
    inputs::key::Key,
    io::IoEvent,
};
use tui_input::{Input, InputRequest};

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
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => self.timeline_action(key).await,
                Tab::Notifications => self.notifications_action(key).await,
            },
            state::Mode::Post => self.post_action(key).await,
            state::Mode::Reply => self.reply_action(key).await,
            state::Mode::Help => self.help_action(key).await,
        }
    }

    async fn timeline_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('r') => {
                self.dispatch(IoEvent::LoadTimeline).await;
                AppReturn::Continue
            }
            Key::Char('n') => {
                self.state.set_mode(state::Mode::Post);
                AppReturn::Continue
            }
            Key::Char('N') => {
                self.state.set_mode(state::Mode::Reply);
                AppReturn::Continue
            }
            Key::Ctrl('r') => {
                self.dispatch(IoEvent::Repost).await;
                AppReturn::Continue
            }
            Key::Ctrl('l') => {
                self.dispatch(IoEvent::Like).await;
                AppReturn::Continue
            }
            Key::Char('?') => {
                self.state.set_mode(state::Mode::Help);
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_tl_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_tl_scroll_up();
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(feed) = self.state.get_current_feed() {
                    if let Some(id) = feed.post.uri.split('/').last() {
                        let handle = feed.post.author.handle;
                        let url = format!("https://bsky.app/profile/{}/post/{}", handle.as_str(), id);
                        let _ = webbrowser::open(&url).is_ok();
                    }
                }
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                self.dispatch(IoEvent::LoadNotifications).await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn notifications_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('r') => {
                self.dispatch(IoEvent::LoadNotifications).await;
                AppReturn::Continue
            }
            Key::Char('?') => {
                self.state.set_mode(state::Mode::Help);
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_notifications_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_notifications_scroll_up();
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                self.dispatch(IoEvent::LoadTimeline).await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn post_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Enter => {
                self.dispatch(IoEvent::SendPost).await;
                AppReturn::Continue
            }
            Key::Left | Key::Ctrl('b') => {
                self.state.move_input_cursor_prev();
                AppReturn::Continue
            }
            Key::Right | Key::Ctrl('f') => {
                self.state.move_input_cursor_next();
                AppReturn::Continue
            }
            Key::Ctrl('a') => {
                self.state.move_input_cursor_start();
                AppReturn::Continue
            }
            Key::Ctrl('e') => {
                self.state.move_input_cursor_end();
                AppReturn::Continue
            }
            Key::Char(c) => {
                self.state.insert_input(InputRequest::InsertChar(c));
                AppReturn::Continue
            }
            Key::Backspace | Key::Ctrl('h') => {
                self.state.remove_input_prev();
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn reply_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Enter => {
                self.dispatch(IoEvent::Reply).await;
                AppReturn::Continue
            }
            Key::Left | Key::Ctrl('b') => {
                self.state.move_input_cursor_prev();
                AppReturn::Continue
            }
            Key::Right | Key::Ctrl('f') => {
                self.state.move_input_cursor_next();
                AppReturn::Continue
            }
            Key::Ctrl('a') => {
                self.state.move_input_cursor_start();
                AppReturn::Continue
            }
            Key::Ctrl('e') => {
                self.state.move_input_cursor_end();
                AppReturn::Continue
            }
            Key::Char(c) => {
                self.state.insert_input(InputRequest::InsertChar(c));
                AppReturn::Continue
            }
            Key::Backspace | Key::Ctrl('h') => {
                self.state.remove_input_prev();
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn help_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Char('?') => {
                self.state.set_mode(state::Mode::Normal);
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
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

    pub fn initialized(
        &mut self,
        agent: bsky::Agent,
        handle: String,
        did: String,
        config: AppConfig,
    ) {
        self.state = AppState::initialized(agent, handle, did, config);
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }
}
