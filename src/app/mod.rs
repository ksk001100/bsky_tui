//! Application model and Elm/Redux-style state-transition boundary.
//!
//! State changes enter through [`App::update`]. Update handlers emit
//! [`command::Command`] values, and asynchronous results return as
//! [`message::EffectMessage`] values.

pub mod auth;
pub mod composer;
pub mod config;
pub mod feature_panel;
pub mod feed;
pub mod images;
pub mod moderation;
pub mod notifications;
pub mod profile;
pub mod state;
pub mod thread;
pub mod ui;
mod update;
pub use update::Update;

pub mod command;
pub mod message;

use atrium_api::types::string::{AtIdentifier, Did, Handle};
use bsky_sdk::BskyAgent;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};
use tui_input::{Input, InputRequest};

use self::state::AppState;
use self::{
    command::{Command, EffectContext},
    images::ImageCache,
};
use crate::{
    app::state::Tab,
    bsky,
    inputs::key::Key,
    io::{
        FeatureEvent, InteractionKind, IoEvent, ModerationAction, NotificationEvent, SearchEvent,
        TimelineEvent,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

struct ImageViewer {
    urls: Vec<String>,
    alt_texts: Vec<String>,
    index: usize,
}

struct FacetViewer {
    facets: Vec<bsky::PostFacet>,
    index: usize,
}

struct InteractionViewer {
    kind: InteractionKind,
    items: Vec<bsky::InteractionItem>,
    index: usize,
}

struct FeedViewer {
    items: Vec<feed::FeedDescriptor>,
    index: usize,
}

struct ActionMenu {
    items: Vec<(&'static str, Key)>,
    index: usize,
}

impl FacetViewer {
    fn new(facets: Vec<bsky::PostFacet>) -> Option<Self> {
        (!facets.is_empty()).then_some(Self { facets, index: 0 })
    }

    fn previous(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    fn next(&mut self) {
        if self.index + 1 < self.facets.len() {
            self.index += 1;
        }
    }
}

impl ImageViewer {
    fn new(urls: Vec<String>, alt_texts: Vec<String>) -> Option<Self> {
        (!urls.is_empty()).then_some(Self {
            urls,
            alt_texts,
            index: 0,
        })
    }

    fn previous(&mut self) {
        self.index = if self.index == 0 {
            self.urls.len() - 1
        } else {
            self.index - 1
        };
    }

    fn next(&mut self) {
        self.index = (self.index + 1) % self.urls.len();
    }
}

pub struct App {
    pending_commands: Vec<Command>,
    is_loading: bool,
    error: Option<String>,
    pub state: AppState,
    images: ImageCache,
    image_viewer: Option<ImageViewer>,
    facet_viewer: Option<FacetViewer>,
    interaction_viewer: Option<InteractionViewer>,
    pending_confirmation: Option<ModerationAction>,
    pending_delete: Option<String>,
    feed_catalog: Vec<feed::FeedDescriptor>,
    feed_viewer: Option<FeedViewer>,
    action_menu: Option<ActionMenu>,
    composer_preview: Option<bsky::LinkPreview>,
    pub(crate) notification_settings: Option<notifications::NotificationSettings>,
    pub(crate) help_table_state: TableState,
    help_return_mode: state::Mode,
    pub(crate) feature_panel: Option<feature_panel::FeaturePanel>,
    pub(crate) messages: feature_panel::FeaturePanel,
    pub(crate) explore: feature_panel::FeaturePanel,
    ui_config: config::UiConfig,
    last_auto_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
        let is_loading = false;
        let state = AppState::default();

        Self {
            pending_commands: Vec::new(),
            is_loading,
            error: None,
            state,
            images: ImageCache::new(),
            image_viewer: None,
            facet_viewer: None,
            interaction_viewer: None,
            pending_confirmation: None,
            pending_delete: None,
            feed_catalog: Vec::new(),
            feed_viewer: None,
            action_menu: None,
            composer_preview: None,
            notification_settings: None,
            help_table_state: TableState::default().with_selected(Some(0)),
            help_return_mode: state::Mode::Normal,
            feature_panel: None,
            messages: feature_panel::FeaturePanel::loading(
                feature_panel::FeatureSection::DirectMessages,
            ),
            explore: feature_panel::FeaturePanel::loading(feature_panel::FeatureSection::Discovery),
            ui_config: config::UiConfig::default(),
            last_auto_refresh: Instant::now(),
        }
    }

    fn open_help(&mut self) {
        self.help_return_mode = self.state.get_mode();
        self.help_table_state.select(Some(0));
        self.state.set_mode(state::Mode::Help);
    }

    fn close_help(&mut self) {
        self.state.set_mode(self.help_return_mode);
    }

    pub(crate) fn content_mode(&self) -> state::Mode {
        if self.state.is_help_mode() {
            self.help_return_mode
        } else {
            self.state.get_mode()
        }
    }

    pub(crate) fn key_hints(&self) -> &'static str {
        if self.error.is_some() {
            return "q/Esc dismiss   Ctrl+C quit";
        }
        if self.pending_confirmation.is_some() || self.pending_delete.is_some() {
            return "y/Enter confirm   q/n/Esc cancel";
        }
        if self.composer_preview.is_some() {
            return "any key close preview";
        }
        if self.notification_settings.is_some() {
            return "↑/↓ category   Space list   p push   i audience   v activity   q/Esc close";
        }
        if self.action_menu.is_some() {
            return "↑/↓ select   Enter run   q/Esc close";
        }
        if let Some(panel) = self.feature_panel.as_ref() {
            if panel.prompt.is_some() {
                return "Enter submit   Esc cancel   ←/→ move cursor";
            }
            return "1 Lists  2 Packs  3 Discover  4 DM  5 Safety  6 Settings   ? help";
        }
        if self.state.is_help_mode() {
            return "↑/↓ move   PgUp/PgDn jump   q/Esc close";
        }
        if self.state.is_feed_search_mode() {
            return "Enter search   Esc cancel";
        }
        if self.feed_viewer.is_some() {
            return "↑/↓ select   Enter choose   / search   s save   q/Esc close";
        }
        if self.image_viewer.is_some() {
            return "←/→ image   q/Esc close";
        }
        if self.facet_viewer.is_some() || self.interaction_viewer.is_some() {
            return "↑/↓ select   Enter open   q/Esc close";
        }
        match self.state.get_mode() {
            state::Mode::Post | state::Mode::Reply => {
                "Ctrl+S send   Enter newline   Ctrl+V preview   Esc cancel"
            }
            state::Mode::Search | state::Mode::UserSearch => "Enter search   Esc cancel",
            state::Mode::Profile => {
                "↑/↓ select   ←/→ section   Enter open   i image   q/Esc back   ? help"
            }
            state::Mode::Thread => {
                "↑/↓ select   H hide reply   M mute thread   Ctrl+D detach quote   q/Esc back"
            }
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => "↑/↓ select   Enter thread   n post   r reply   Tab switch   ? help",
                Tab::Notifications => {
                    "↑/↓ select   1/2/3 filter   p settings   f follow   L like   a profile"
                }
                Tab::Messages => "↑/↓ select   Enter open   n new   w write   F5 reload   ? help",
                Tab::Search => {
                    if self.state.get_search_query().is_some() {
                        "↑/↓ select   Enter thread   Esc explore   / search   Tab switch   ? help"
                    } else {
                        "↑/↓ select   Enter explore   / search   u users   Tab switch   ? help"
                    }
                }
            },
            state::Mode::Help => "↑/↓ move   PgUp/PgDn jump   q/Esc close",
            state::Mode::FeedSearch => "Enter search   Esc cancel",
        }
    }

    pub(in crate::app) fn handle_key(&mut self, key: Key) -> AppReturn {
        // Ctrl+C is the only unconditional application exit. q acts like Esc
        // outside text entry, closing the current layer or going back.
        if key == Key::Ctrl('c') {
            return AppReturn::Exit;
        }

        if self.is_loading {
            return AppReturn::Continue;
        }

        if !self.state.is_initialized() {
            return match key {
                Key::Char('r') => {
                    self.error = None;
                    self.dispatch(IoEvent::Initialize);
                    AppReturn::Continue
                }
                Key::Char('q') | Key::Esc => {
                    self.error = None;
                    AppReturn::Continue
                }
                _ => AppReturn::Continue,
            };
        }

        let text_entry_active = matches!(
            self.state.get_mode(),
            state::Mode::Post
                | state::Mode::Reply
                | state::Mode::Search
                | state::Mode::UserSearch
                | state::Mode::FeedSearch
        ) || self
            .feature_panel
            .as_ref()
            .is_some_and(|panel| panel.prompt.is_some())
            || self.messages.prompt.is_some();
        let key = close_or_back_key(key, text_entry_active);

        if self.error.is_some() {
            if key == Key::Esc {
                self.error = None;
            }
            return AppReturn::Continue;
        }

        if self.pending_confirmation.is_some() || self.pending_delete.is_some() {
            return self.confirmation_action(key);
        }
        if self.composer_preview.is_some() {
            self.composer_preview = None;
            return AppReturn::Continue;
        }
        if self.notification_settings.is_some() {
            return self.notification_settings_action(key);
        }
        if self.action_menu.is_some() {
            return self.action_menu_action(key);
        }
        if self.feature_panel.is_some() {
            return self.feature_panel_action(key);
        }
        if self.feed_viewer.is_some() && !self.state.is_feed_search_mode() {
            return self.feed_viewer_action(key);
        }

        if self.image_viewer.is_some() {
            return self.image_viewer_action(key);
        }
        if self.facet_viewer.is_some() {
            return self.facet_viewer_action(key);
        }
        if self.interaction_viewer.is_some() {
            return self.interaction_viewer_action(key);
        }

        if self.state.get_mode() == state::Mode::Normal {
            if self.configured_key("action_menu", key, Key::Char(':')) {
                self.action_menu = Some(ActionMenu {
                    items: action_items(
                        self.state.get_tab(),
                        self.state.get_search_query().is_some(),
                    ),
                    index: 0,
                });
                return AppReturn::Continue;
            }
            if self.configured_key("open_dm", key, Key::Char('d')) {
                self.state.set_tab(Tab::Messages);
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::DirectMessages,
                )));
                return AppReturn::Continue;
            }
            let section = if self.configured_key("open_lists", key, Key::Char('g')) {
                Some(feature_panel::FeatureSection::Lists)
            } else if self.configured_key("open_moderation", key, Key::Char(';')) {
                Some(feature_panel::FeatureSection::Moderation)
            } else if self.configured_key("open_settings", key, Key::Char(',')) {
                Some(feature_panel::FeatureSection::Settings)
            } else {
                None
            };
            if let Some(section) = section {
                self.feature_panel = Some(feature_panel::FeaturePanel::loading(section));
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(section)));
                return AppReturn::Continue;
            }
        }

        let key = if matches!(
            self.state.get_mode(),
            state::Mode::Normal | state::Mode::Thread | state::Mode::Profile
        ) {
            self.remap_navigation_key(key)
        } else {
            key
        };

        match self.state.get_mode() {
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => self.timeline_action(key),
                Tab::Notifications => self.notifications_action(key),
                Tab::Messages => self.messages_action(key),
                Tab::Search => self.search_action(key),
            },
            state::Mode::Post => self.post_action(key),
            state::Mode::Reply => self.reply_action(key),
            state::Mode::Help => self.help_action(key),
            state::Mode::Search => self.search_input_action(key),
            state::Mode::UserSearch => self.user_search_input_action(key),
            state::Mode::Thread => self.thread_action(key),
            state::Mode::Profile => self.profile_action(key),
            state::Mode::FeedSearch => self.feed_search_input_action(key),
        }
    }

    pub(in crate::app) fn handle_mouse(&mut self, mouse: MouseEvent) -> AppReturn {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.move_current_selection(-3),
            MouseEventKind::ScrollDown => self.move_current_selection(3),
            MouseEventKind::Down(MouseButton::Left) if mouse.row <= 6 => {
                let width = crossterm::terminal::size().map_or(1, |size| size.0.max(1));
                let tab = match mouse.column.saturating_mul(4) / width {
                    0 => Tab::Home,
                    1 => Tab::Notifications,
                    2 => Tab::Messages,
                    _ => Tab::Search,
                };
                if self.state.get_mode() == state::Mode::Normal {
                    self.state.set_tab(tab);
                    match tab {
                        Tab::Home if self.state.get_timeline().is_none() => {
                            self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Load))
                        }
                        Tab::Notifications => {
                            self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load))
                        }
                        Tab::Messages => self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                            feature_panel::FeatureSection::DirectMessages,
                        ))),
                        Tab::Search if self.state.get_search_query().is_none() => {
                            self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                                feature_panel::FeatureSection::Discovery,
                            )))
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    fn move_current_selection(&mut self, delta: isize) {
        match self.state.get_mode() {
            state::Mode::Thread => self.state.move_thread_by(delta),
            state::Mode::Profile => self.state.move_profile_by(delta),
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => self.state.move_tl_scroll_by(delta),
                Tab::Notifications => self.state.move_notifications_scroll_by(delta),
                Tab::Messages => {
                    if delta < 0 {
                        for _ in 0..delta.unsigned_abs() {
                            self.messages.previous();
                        }
                    } else {
                        for _ in 0..delta as usize {
                            self.messages.next();
                        }
                    }
                }
                Tab::Search => {
                    if self.state.get_search_query().is_some() {
                        self.state.move_search_scroll_by(delta);
                    } else if delta < 0 {
                        for _ in 0..delta.unsigned_abs() {
                            self.explore.previous();
                        }
                    } else {
                        for _ in 0..delta as usize {
                            self.explore.next();
                        }
                    }
                }
            },
            _ => {}
        }
    }

    fn selected_post(&self) -> Option<atrium_api::app::bsky::feed::defs::PostViewData> {
        match self.state.get_mode() {
            state::Mode::Thread => self.state.get_current_thread_post(),
            state::Mode::Profile => self
                .state
                .get_current_profile_post()
                .map(|feed| feed.post.data.clone()),
            _ => match self.state.get_tab() {
                Tab::Home => self
                    .state
                    .get_current_feed()
                    .map(|feed| feed.post.data.clone()),
                Tab::Search if self.state.get_search_query().is_some() => {
                    self.state.get_current_search_result()
                }
                Tab::Search => None,
                Tab::Notifications => self.state.get_current_notification_post(),
                Tab::Messages => None,
            },
        }
    }

    fn copy_selected_post(&mut self, key: Key) {
        let Some(post) = self.selected_post() else {
            self.set_error("No post is selected".into());
            return;
        };
        let (label, value) = match key {
            Key::Char('y') => ("post text", bsky::post_text(&post).unwrap_or_default()),
            Key::Char('Y') => (
                "post URL",
                bsky::get_url(post.author.handle.clone(), post.uri.clone()).unwrap_or(post.uri),
            ),
            _ => (
                "AT URI and DID",
                format!("{}\n{}", post.uri, post.author.did.as_str()),
            ),
        };
        if value.is_empty() {
            self.set_error(format!("Selected {label} is empty"));
        } else {
            self.pending_commands
                .push(Command::CopyToClipboard { value, label });
        }
    }

    fn preview_composer_link(&mut self) {
        match composer::parse_drafts(self.state.get_input().value())
            .ok()
            .and_then(|drafts| drafts.into_iter().find_map(|draft| draft.link))
        {
            Some(url) => self.dispatch(IoEvent::PreviewLink(url)),
            None => self.set_error("Add a !link URL directive before previewing".to_owned()),
        }
    }

    pub fn set_composer_preview(&mut self, preview: bsky::LinkPreview) {
        self.composer_preview = Some(preview);
    }

    pub fn composer_preview(&self) -> Option<bsky::LinkPreview> {
        self.composer_preview.clone()
    }

    pub fn set_interactions(&mut self, kind: InteractionKind, items: Vec<bsky::InteractionItem>) {
        self.interaction_viewer = Some(InteractionViewer {
            kind,
            items,
            index: 0,
        });
    }

    pub fn set_interactions_closed(&mut self) {
        self.interaction_viewer = None;
    }

    pub fn current_interactions(
        &self,
    ) -> Option<(InteractionKind, Vec<bsky::InteractionItem>, usize)> {
        let viewer = self.interaction_viewer.as_ref()?;
        Some((viewer.kind, viewer.items.clone(), viewer.index))
    }

    fn open_image_viewer(&mut self, urls: Vec<String>, alt_texts: Vec<String>) {
        self.queue_images(urls.clone());
        self.image_viewer = ImageViewer::new(urls, alt_texts);
    }

    pub fn current_image_viewer(&self) -> Option<(String, String, usize, usize)> {
        let viewer = self.image_viewer.as_ref()?;
        Some((
            viewer.urls[viewer.index].clone(),
            viewer
                .alt_texts
                .get(viewer.index)
                .cloned()
                .unwrap_or_default(),
            viewer.index + 1,
            viewer.urls.len(),
        ))
    }

    pub(in crate::app) fn update_on_tick(&mut self) -> AppReturn {
        self.pending_commands.push(Command::PollImages);
        let interval = self.ui_config.auto_refresh_seconds;
        if interval > 0
            && !self.is_loading
            && self.error.is_none()
            && self.state.get_mode() == state::Mode::Normal
            && self.last_auto_refresh.elapsed() >= Duration::from_secs(interval)
        {
            self.last_auto_refresh = Instant::now();
            match self.state.get_tab() {
                Tab::Home => self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload)),
                Tab::Notifications => {
                    self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Reload))
                }
                Tab::Messages => {
                    if let Some(convo_id) = self
                        .messages
                        .title
                        .strip_prefix("Conversation · ")
                        .map(str::to_owned)
                    {
                        self.dispatch(IoEvent::Feature(FeatureEvent::OpenConversation(convo_id)))
                    } else {
                        self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                            feature_panel::FeatureSection::DirectMessages,
                        )))
                    }
                }
                Tab::Search if self.state.get_search_query().is_some() => {
                    self.dispatch(IoEvent::Search(SearchEvent::Reload))
                }
                Tab::Search => self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::Discovery,
                ))),
            }
        }
        AppReturn::Continue
    }

    pub fn configure_images(&mut self, picker: ratatui_image::picker::Picker) {
        self.images.configure(picker);
    }

    pub(crate) fn queue_images<I>(&mut self, urls: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.images.queue(urls);
    }

    pub(in crate::app) fn defer_images(&mut self, urls: Vec<String>) {
        if !urls.is_empty() {
            self.pending_commands.push(Command::LoadImages(urls));
        }
    }

    pub(in crate::app) fn open_url(&mut self, url: String, error_context: &'static str) {
        self.pending_commands
            .push(Command::OpenUrl { url, error_context });
    }

    pub(crate) fn poll_images(&mut self) {
        self.images.poll();
    }

    pub fn image_mut(
        &mut self,
        url: &str,
    ) -> Option<&mut ratatui_image::protocol::StatefulProtocol> {
        self.images.get_mut(url)
    }

    pub fn centered_image_area(
        &self,
        url: &str,
        bounds: ratatui::layout::Rect,
    ) -> Option<ratatui::layout::Rect> {
        self.images.centered_area(url, bounds)
    }

    pub(in crate::app) fn dispatch(&mut self, action: IoEvent) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        if matches!(action, IoEvent::LoadTimeline(_)) {
            self.last_auto_refresh = Instant::now();
        }
        let context = self.effect_context(&action);
        self.pending_commands.push(Command::Io {
            event: action,
            context: Box::new(context),
        });
    }

    pub fn effect_context(&self, event: &IoEvent) -> EffectContext {
        EffectContext::for_event(
            &self.state,
            event,
            self.feature_panel.is_some(),
            self.feature_panel.as_ref().map(|panel| panel.section),
        )
    }

    pub(in crate::app) fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.pending_commands)
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn input_cursor_position(&self) -> (u16, u16) {
        use unicode_width::UnicodeWidthStr;

        let input = self.state.get_input();
        let before_cursor = input
            .value()
            .chars()
            .take(input.cursor())
            .collect::<String>();
        let row = before_cursor
            .chars()
            .filter(|character| *character == '\n')
            .count();
        let column = before_cursor
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .width();
        (column as u16, row as u16)
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn initialized(
        &mut self,
        agent: BskyAgent,
        handle: Handle,
        did: Did,
        moderation: moderation::ModerationPrefs,
        ui_config: config::UiConfig,
    ) {
        self.state = AppState::initialized(agent, handle, did, moderation);
        self.ui_config = ui_config;
    }

    pub fn images_enabled(&self) -> bool {
        self.ui_config.show_images
    }

    pub fn accent_color(&self) -> ratatui::style::Color {
        match self.ui_config.accent_color.to_ascii_lowercase().as_str() {
            "red" => ratatui::style::Color::LightRed,
            "green" => ratatui::style::Color::LightGreen,
            "yellow" => ratatui::style::Color::LightYellow,
            "magenta" => ratatui::style::Color::LightMagenta,
            "cyan" => ratatui::style::Color::LightCyan,
            "white" => ratatui::style::Color::White,
            _ => ratatui::style::Color::LightBlue,
        }
    }

    pub fn set_ui_config(&mut self, ui_config: config::UiConfig) {
        self.ui_config = ui_config;
    }

    fn configured_key(&self, action: &str, key: Key, default: Key) -> bool {
        self.ui_config
            .keybindings
            .get(action)
            .map(|binding| binding_matches(binding, key))
            .unwrap_or(key == default)
    }

    fn remap_navigation_key(&self, key: Key) -> Key {
        const ACTIONS: &[(&str, Key)] = &[
            ("move_up", Key::Up),
            ("move_down", Key::Down),
            ("half_page_up", Key::PageUp),
            ("half_page_down", Key::PageDown),
            ("first_item", Key::Home),
            ("last_item", Key::End),
            ("reload", Key::F5),
            ("open", Key::Enter),
            ("copy_text", Key::Char('y')),
            ("copy_url", Key::Char('Y')),
            ("copy_ids", Key::Alt('y')),
        ];
        ACTIONS
            .iter()
            .find_map(|(action, canonical)| {
                self.ui_config
                    .keybindings
                    .get(*action)
                    .filter(|binding| binding_matches(binding, key))
                    .map(|_| *canonical)
            })
            .unwrap_or(key)
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn interaction_kind(key: Key) -> InteractionKind {
    match key {
        Key::Char('L') => InteractionKind::Likes,
        Key::Char('R') => InteractionKind::Reposts,
        _ => InteractionKind::Quotes,
    }
}

fn close_or_back_key(key: Key, text_entry_active: bool) -> Key {
    if key == Key::Char('q') && !text_entry_active {
        Key::Esc
    } else {
        key
    }
}

fn action_items(tab: Tab, has_search_results: bool) -> Vec<(&'static str, Key)> {
    if tab == Tab::Messages {
        return vec![
            ("Reload messages", Key::F5),
            ("Open conversation", Key::Enter),
            ("New conversation", Key::Char('n')),
            ("Write message", Key::Char('w')),
            ("Mute or unmute conversation", Key::Char(' ')),
        ];
    }
    if tab == Tab::Search && !has_search_results {
        return vec![
            ("Reload", Key::F5),
            ("Open selected item", Key::Enter),
            ("Search posts", Key::Char('/')),
            ("Search users", Key::Char('u')),
            ("First item", Key::Home),
            ("Last item", Key::End),
        ];
    }
    let mut items = vec![
        ("Reload", Key::F5),
        ("Open selected item", Key::Enter),
        ("Copy post text", Key::Char('y')),
        ("Copy post URL", Key::Char('Y')),
        ("Copy AT URI and DID", Key::Alt('y')),
        ("First item", Key::Home),
        ("Last item", Key::End),
    ];
    if tab == Tab::Home {
        items.extend([("New post", Key::Char('n')), ("Reply", Key::Char('r'))]);
    }
    items
}

pub(crate) fn copy_osc52(value: &str) -> std::io::Result<()> {
    use std::io::Write;
    let encoded = base64_encode(value.as_bytes());
    let mut stdout = std::io::stdout();
    write!(stdout, "\x1b]52;c;{encoded}\x07")?;
    stdout.flush()
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(TABLE[((bits >> 18) & 63) as usize] as char);
        output.push(TABLE[((bits >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn actor_moderation_action(
    actor: &atrium_api::app::bsky::actor::defs::ProfileViewBasicData,
    key: Key,
) -> ModerationAction {
    let viewer = actor.viewer.as_ref();
    match key {
        Key::Char('m') => ModerationAction::MuteActor {
            did: actor.did.clone(),
            muted: viewer.and_then(|viewer| viewer.muted).unwrap_or(false),
        },
        _ => ModerationAction::BlockActor {
            did: actor.did.clone(),
            blocking_uri: viewer.and_then(|viewer| viewer.blocking.clone()),
        },
    }
}

fn binding_matches(binding: &str, key: Key) -> bool {
    let binding = binding.trim().to_ascii_lowercase();
    match key {
        Key::Char(character) => binding == character.to_string(),
        Key::Ctrl(character) => binding == format!("ctrl+{character}"),
        Key::Alt(character) => binding == format!("alt+{character}"),
        Key::Enter => binding == "enter",
        Key::Esc => binding == "esc",
        Key::Tab => binding == "tab",
        Key::Up => binding == "up",
        Key::Down => binding == "down",
        Key::Left => binding == "left",
        Key::Right => binding == "right",
        Key::Home => binding == "home",
        Key::End => binding == "end",
        Key::PageUp => binding == "pageup" || binding == "page_up",
        Key::PageDown => binding == "pagedown" || binding == "page_down",
        Key::F1 => binding == "f1",
        Key::F5 => binding == "f5",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_viewer_navigation_wraps_in_both_directions() {
        let mut viewer = ImageViewer::new(
            vec!["one".into(), "two".into(), "three".into()],
            vec!["first".into(), "second".into(), "third".into()],
        )
        .expect("viewer should open");

        viewer.previous();
        assert_eq!(viewer.index, 2);
        viewer.next();
        assert_eq!(viewer.index, 0);
        viewer.next();
        assert_eq!(viewer.index, 1);
    }

    #[test]
    fn image_viewer_does_not_open_without_attachments() {
        assert!(ImageViewer::new(Vec::new(), Vec::new()).is_none());
    }

    #[test]
    fn facet_viewer_stays_within_bounds() {
        let mut viewer = FacetViewer::new(vec![
            bsky::PostFacet {
                label: "one".into(),
                kind: "URL",
                url: "https://one.example".into(),
            },
            bsky::PostFacet {
                label: "two".into(),
                kind: "URL",
                url: "https://two.example".into(),
            },
        ])
        .expect("viewer should open");
        viewer.previous();
        assert_eq!(viewer.index, 0);
        viewer.next();
        viewer.next();
        assert_eq!(viewer.index, 1);
    }

    #[test]
    fn help_navigation_moves_and_stays_within_bounds() {
        let mut app = App::new();

        app.help_action(Key::Up);
        assert_eq!(app.help_table_state.selected(), Some(0));

        app.help_action(Key::PageDown);
        assert_eq!(app.help_table_state.selected(), Some(10));

        app.help_action(Key::End);
        app.help_action(Key::Down);
        assert_eq!(
            app.help_table_state.selected(),
            Some(ui::HELP_ROW_COUNT - 1)
        );

        app.help_action(Key::Home);
        assert_eq!(app.help_table_state.selected(), Some(0));
    }

    #[test]
    fn osc52_base64_supports_unicode_clipboard_content() {
        assert_eq!(base64_encode("日本".as_bytes()), "5pel5pys");
        assert!(binding_matches("page_down", Key::PageDown));
    }

    #[test]
    fn q_is_close_or_back_except_during_text_entry() {
        assert_eq!(close_or_back_key(Key::Char('q'), false), Key::Esc);
        assert_eq!(close_or_back_key(Key::Char('q'), true), Key::Char('q'));
    }
}
