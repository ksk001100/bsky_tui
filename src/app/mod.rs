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

use atrium_api::types::string::{AtIdentifier, Did, Handle};
use bsky_sdk::BskyAgent;
use ratatui::widgets::TableState;
use tui_input::{Input, InputRequest};

use self::images::ImageCache;
use self::state::AppState;
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
    io_tx: tokio::sync::mpsc::Sender<IoEvent>,
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
    composer_preview: Option<bsky::LinkPreview>,
    pub(crate) notification_settings: Option<notifications::NotificationSettings>,
    pub(crate) help_table_state: TableState,
    help_return_mode: state::Mode,
    pub(crate) feature_panel: Option<feature_panel::FeaturePanel>,
    ui_config: config::UiConfig,
}

impl App {
    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let is_loading = false;
        let state = AppState::default();

        Self {
            io_tx,
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
            composer_preview: None,
            notification_settings: None,
            help_table_state: TableState::default().with_selected(Some(0)),
            help_return_mode: state::Mode::Normal,
            feature_panel: None,
            ui_config: config::UiConfig::default(),
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
            return "Esc dismiss   q quit";
        }
        if self.pending_confirmation.is_some() || self.pending_delete.is_some() {
            return "y/Enter confirm   Esc cancel";
        }
        if self.composer_preview.is_some() {
            return "any key close preview";
        }
        if self.notification_settings.is_some() {
            return "↑/↓ category   Space list   p push   i audience   v activity   Esc close";
        }
        if let Some(panel) = self.feature_panel.as_ref() {
            if panel.prompt.is_some() {
                return "Enter submit   Esc cancel   ←/→ move cursor";
            }
            return "1 Lists  2 Packs  3 Discover  4 DM  5 Safety  6 Settings   ? help";
        }
        if self.state.is_help_mode() {
            return "↑/↓ move   PgUp/PgDn jump   Esc close";
        }
        if self.state.is_feed_search_mode() {
            return "Enter search   Esc cancel";
        }
        if self.feed_viewer.is_some() {
            return "↑/↓ select   Enter choose   / search   s save   Esc close";
        }
        if self.image_viewer.is_some() {
            return "←/→ image   Esc close";
        }
        if self.facet_viewer.is_some() || self.interaction_viewer.is_some() {
            return "↑/↓ select   Enter open   Esc close";
        }
        match self.state.get_mode() {
            state::Mode::Post | state::Mode::Reply => {
                "Ctrl+S send   Enter newline   Ctrl+V preview   Esc cancel"
            }
            state::Mode::Search | state::Mode::UserSearch => "Enter search   Esc cancel",
            state::Mode::Profile => {
                "↑/↓ select   ←/→ section   Enter open   i image   Esc back   ? help"
            }
            state::Mode::Thread => {
                "↑/↓ select   H hide reply   M mute thread   Ctrl+D detach quote   Esc back"
            }
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => "↑/↓ select   Enter thread   n post   r reply   Tab switch   ? help",
                Tab::Notifications => {
                    "↑/↓ select   1/2/3 filter   p settings   f follow   L like   a profile"
                }
                Tab::Search => {
                    "↑/↓ select   Enter thread   i image   / search   Tab switch   ? help"
                }
            },
            state::Mode::Help | state::Mode::FeedSearch => unreachable!(),
        }
    }

    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        if self.is_loading {
            return match key {
                Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
                _ => AppReturn::Continue,
            };
        }

        if !self.state.is_initialized() {
            return match key {
                Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
                Key::Char('r') => {
                    self.error = None;
                    self.dispatch(IoEvent::Initialize).await;
                    AppReturn::Continue
                }
                _ => AppReturn::Continue,
            };
        }

        // Ctrl+C is the unconditional emergency exit. Esc is reserved for
        // cancelling or closing the current layer.
        if key == Key::Ctrl('c') {
            return AppReturn::Exit;
        }
        if key == Key::Char('q')
            && !matches!(
                self.state.get_mode(),
                state::Mode::Post
                    | state::Mode::Reply
                    | state::Mode::Search
                    | state::Mode::UserSearch
                    | state::Mode::FeedSearch
            )
        {
            return AppReturn::Exit;
        }

        if self.error.is_some() {
            if key == Key::Esc {
                self.error = None;
            }
            return AppReturn::Continue;
        }

        if self.pending_confirmation.is_some() || self.pending_delete.is_some() {
            return self.confirmation_action(key).await;
        }
        if self.composer_preview.is_some() {
            self.composer_preview = None;
            return AppReturn::Continue;
        }
        if self.notification_settings.is_some() {
            return self.notification_settings_action(key).await;
        }
        if self.feature_panel.is_some() {
            return self.feature_panel_action(key).await;
        }
        if self.feed_viewer.is_some() && !self.state.is_feed_search_mode() {
            return self.feed_viewer_action(key).await;
        }

        if self.image_viewer.is_some() {
            return self.image_viewer_action(key);
        }
        if self.facet_viewer.is_some() {
            return self.facet_viewer_action(key);
        }
        if self.interaction_viewer.is_some() {
            return self.interaction_viewer_action(key).await;
        }

        if self.state.get_mode() == state::Mode::Normal {
            let section = if self.configured_key("open_lists", key, Key::Char('g')) {
                Some(feature_panel::FeatureSection::Lists)
            } else if self.configured_key("open_dm", key, Key::Char('d')) {
                Some(feature_panel::FeatureSection::DirectMessages)
            } else if self.configured_key("open_moderation", key, Key::Char(';')) {
                Some(feature_panel::FeatureSection::Moderation)
            } else if self.configured_key("open_settings", key, Key::Char(',')) {
                Some(feature_panel::FeatureSection::Settings)
            } else {
                None
            };
            if let Some(section) = section {
                self.feature_panel = Some(feature_panel::FeaturePanel::loading(section));
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(section)))
                    .await;
                return AppReturn::Continue;
            }
        }

        match self.state.get_mode() {
            state::Mode::Normal => match self.state.get_tab() {
                Tab::Home => self.timeline_action(key).await,
                Tab::Notifications => self.notifications_action(key).await,
                Tab::Search => self.search_action(key).await,
            },
            state::Mode::Post => self.post_action(key).await,
            state::Mode::Reply => self.reply_action(key).await,
            state::Mode::Help => self.help_action(key).await,
            state::Mode::Search => self.search_input_action(key).await,
            state::Mode::UserSearch => self.user_search_input_action(key).await,
            state::Mode::Thread => self.thread_action(key).await,
            state::Mode::Profile => self.profile_action(key).await,
            state::Mode::FeedSearch => self.feed_search_input_action(key).await,
        }
    }

    async fn timeline_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::F5 => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload))
                    .await;
                AppReturn::Continue
            }
            Key::Char('c') => {
                if self.feed_catalog.is_empty() {
                    self.dispatch(IoEvent::LoadFeedCatalog).await;
                } else {
                    self.feed_viewer = Some(FeedViewer {
                        items: self.feed_catalog.clone(),
                        index: 0,
                    });
                }
                AppReturn::Continue
            }
            Key::Char('n') => {
                self.state.set_mode(state::Mode::Post);
                AppReturn::Continue
            }
            Key::Char('r') => {
                self.state.set_mode(state::Mode::Reply);
                AppReturn::Continue
            }
            Key::Char('X') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.start_quote_composer(&feed.post);
                }
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
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::Search);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('u') => {
                self.state.set_mode(state::Mode::UserSearch);
                self.state.set_input(Input::default());
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
            Key::Char('o') => {
                if let Some(feed) = self.state.get_current_feed() {
                    if let Some(id) = feed.post.uri.split('/').next_back() {
                        let handle = &feed.post.author.handle;
                        let url =
                            format!("https://bsky.app/profile/{}/post/{}", handle.as_str(), id);
                        let _ = webbrowser::open(&url).is_ok();
                    }
                }
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::LoadThread(feed.post.uri.clone()))
                        .await;
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::LoadProfile(feed.post.author.did.clone().into()))
                        .await;
                }
                AppReturn::Continue
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.request_post_moderation(&feed.post, key);
                }
                AppReturn::Continue
            }
            Key::Char('D') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.request_delete(&feed.post);
                }
                AppReturn::Continue
            }
            Key::Char('e') => {
                self.open_selected_embed(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Char('f') => {
                self.open_facet_viewer(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(feed) = self.state.get_current_feed() {
                    let kind = interaction_kind(key);
                    self.dispatch(IoEvent::LoadInteractions(
                        kind,
                        feed.post.uri.clone(),
                        feed.post.cid.clone(),
                    ))
                    .await;
                }
                AppReturn::Continue
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                match self.state.get_tab() {
                    Tab::Home => {
                        self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload))
                            .await;
                    }
                    Tab::Notifications => {
                        self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load))
                            .await;
                    }
                    Tab::Search => {}
                }
                AppReturn::Continue
            }
            Key::Char('h') | Key::Left | Key::Char('[') | Key::PageUp => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Prev))
                    .await;
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') | Key::PageDown => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Next))
                    .await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn notifications_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::F5 => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Reload))
                    .await;
                AppReturn::Continue
            }
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::Search);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('u') => {
                self.state.set_mode(state::Mode::UserSearch);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('1') => {
                self.state.cycle_notification_reason_filter();
                AppReturn::Continue
            }
            Key::Char('2') => {
                self.state.cycle_notification_sender_filter();
                AppReturn::Continue
            }
            Key::Char('3') => {
                self.state.cycle_notification_read_filter();
                AppReturn::Continue
            }
            Key::Char('p') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LoadNotificationSettings(
                        notification.author.did.clone(),
                        notification.author.handle.to_string(),
                    ))
                    .await;
                }
                AppReturn::Continue
            }
            Key::Char('f') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::ToggleNotificationFollow(
                        notification.author.did.clone(),
                    ))
                    .await;
                }
                AppReturn::Continue
            }
            Key::Char('L') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LikeNotificationAuthor(
                        notification.author.did.clone(),
                    ))
                    .await;
                }
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
            Key::Enter | Key::Char('o') => {
                let url = self
                    .state
                    .get_current_notification()
                    .and_then(|notification| {
                        bsky::notification_post_url(&notification, &self.state.get_handle())
                    });
                match url {
                    Some(url) => {
                        if let Err(error) = webbrowser::open(&url) {
                            self.set_error(format!("Could not open notification: {error}"));
                        }
                    }
                    None => self.set_error(
                        "This notification does not refer to a post that can be opened".to_owned(),
                    ),
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LoadProfile(notification.author.did.clone().into()))
                        .await;
                }
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                match self.state.get_tab() {
                    Tab::Home => {
                        self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload))
                            .await;
                    }
                    Tab::Notifications => {
                        self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load))
                            .await;
                    }
                    Tab::Search => {}
                }
                AppReturn::Continue
            }
            Key::Char('h') | Key::Left | Key::Char('[') | Key::PageUp => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Prev))
                    .await;
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') | Key::PageDown => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Next))
                    .await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn notification_settings_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.notification_settings = None,
            Key::Up | Key::Char('k') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.previous();
                }
            }
            Key::Down | Key::Char('j') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.next();
                }
            }
            Key::Char(' ') | Key::Char('p') | Key::Char('i') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    match key {
                        Key::Char(' ') => settings.toggle_list(),
                        Key::Char('p') => settings.toggle_push(),
                        Key::Char('i') => settings.cycle_include(),
                        _ => {}
                    }
                    let preferences = settings.preferences.clone();
                    self.dispatch(IoEvent::SaveNotificationPreferences(Box::new(preferences)))
                        .await;
                }
            }
            Key::Char('v') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.cycle_activity();
                    if let Some((subject, _, activity)) = settings.activity_subject.clone() {
                        self.dispatch(IoEvent::SaveActivitySubscription { subject, activity })
                            .await;
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    async fn search_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::F5 => {
                self.dispatch(IoEvent::Search(SearchEvent::Reload)).await;
                AppReturn::Continue
            }
            Key::Char('r') => {
                self.state.set_tab(Tab::Search);
                self.state.set_mode(state::Mode::Reply);
                AppReturn::Continue
            }
            Key::Char('X') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.start_quote_composer(&post);
                }
                AppReturn::Continue
            }
            Key::Ctrl('r') => {
                self.dispatch(IoEvent::SearchRepost).await;
                AppReturn::Continue
            }
            Key::Ctrl('l') => {
                self.dispatch(IoEvent::SearchLike).await;
                AppReturn::Continue
            }
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::Search);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('u') => {
                self.state.set_mode(state::Mode::UserSearch);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_search_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_search_scroll_up();
                AppReturn::Continue
            }
            Key::Char('o') => {
                if let Some(feed) = self.state.get_current_search_result() {
                    if let Some(id) = feed.uri.split('/').next_back() {
                        let handle = &feed.author.handle;
                        let url =
                            format!("https://bsky.app/profile/{}/post/{}", handle.as_str(), id);
                        let _ = webbrowser::open(&url).is_ok();
                    }
                }
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadThread(post.uri.clone())).await;
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadProfile(post.author.did.clone().into()))
                        .await;
                }
                AppReturn::Continue
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.request_post_moderation(&post, key);
                }
                AppReturn::Continue
            }
            Key::Char('D') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.request_delete(&post);
                }
                AppReturn::Continue
            }
            Key::Char('e') => {
                self.open_selected_embed(self.state.get_current_search_result());
                AppReturn::Continue
            }
            Key::Char('f') => {
                self.open_facet_viewer(self.state.get_current_search_result());
                AppReturn::Continue
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadInteractions(
                        interaction_kind(key),
                        post.uri,
                        post.cid,
                    ))
                    .await;
                }
                AppReturn::Continue
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(self.state.get_current_search_result());
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                match self.state.get_tab() {
                    Tab::Home => {
                        self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload))
                            .await;
                    }
                    Tab::Notifications => {
                        self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load))
                            .await;
                    }
                    Tab::Search => {}
                }
                AppReturn::Continue
            }
            Key::Char('h') | Key::Left | Key::Char('[') | Key::PageUp => {
                match self.state.get_search_query() {
                    Some(_) => {
                        self.dispatch(IoEvent::Search(SearchEvent::Prev)).await;
                    }
                    None => {
                        let query = self.state.get_input().value().to_string();
                        self.state.set_search_query(Some(query.clone()));
                        self.dispatch(IoEvent::Search(SearchEvent::Prev)).await;
                    }
                }
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') | Key::PageDown => {
                match self.state.get_search_query() {
                    Some(_) => {
                        self.dispatch(IoEvent::Search(SearchEvent::Next)).await;
                    }
                    None => {
                        let query = self.state.get_input().value().to_string();
                        self.state.set_search_query(Some(query.clone()));
                        self.dispatch(IoEvent::Search(SearchEvent::Next)).await;
                    }
                }
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn profile_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.state.close_profile(),
            Key::Char('?') | Key::F1 => self.open_help(),
            Key::Down | Key::Char('j') | Key::Ctrl('n') => self.state.move_profile_down(),
            Key::Up | Key::Char('k') | Key::Ctrl('p') => self.state.move_profile_up(),
            Key::Left | Key::Char('h') => {
                if let Some(profile) = self.state.get_profile() {
                    let section = profile.section.previous();
                    if section != profile.section {
                        self.dispatch(IoEvent::LoadProfileSection(section)).await;
                    }
                }
            }
            Key::Right | Key::Char('l') => {
                if let Some(profile) = self.state.get_profile() {
                    let section = profile.section.next();
                    if section != profile.section {
                        self.dispatch(IoEvent::LoadProfileSection(section)).await;
                    }
                }
            }
            Key::Char('F') => self.dispatch(IoEvent::ToggleFollow).await,
            Key::Char('X') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.start_quote_composer(&feed.post);
                }
            }
            Key::Char('D') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.request_delete(&feed.post);
                }
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(profile) = self.state.get_profile() {
                    self.request_profile_moderation(&profile.details, key);
                }
            }
            Key::Char('g') | Key::Char('G') => {
                if let Some(profile) = self.state.get_profile() {
                    let kind = if key == Key::Char('g') {
                        InteractionKind::Followers
                    } else {
                        InteractionKind::Follows
                    };
                    self.dispatch(IoEvent::LoadConnections(
                        kind,
                        profile.details.did.clone().into(),
                    ))
                    .await;
                }
            }
            Key::Char('t') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.dispatch(IoEvent::LoadThread(feed.post.uri.clone()))
                        .await;
                }
            }
            Key::Char('a') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.dispatch(IoEvent::LoadProfile(feed.post.author.did.clone().into()))
                        .await;
                }
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(
                    self.state
                        .get_current_profile_post()
                        .map(|feed| feed.post.data.clone()),
                );
            }
            Key::Enter | Key::Char('o') => {
                let url = self
                    .state
                    .get_current_profile_post()
                    .and_then(|feed| {
                        bsky::get_url(feed.post.author.handle.clone(), feed.post.uri.clone())
                    })
                    .or_else(|| self.state.get_current_profile_item().map(|item| item.url));
                if let Some(url) = url {
                    if let Err(error) = webbrowser::open(&url) {
                        self.set_error(format!("Could not open profile item: {error}"));
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    async fn thread_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.state.close_thread(),
            Key::Char('?') | Key::F1 => self.open_help(),
            Key::Down | Key::Char('j') | Key::Ctrl('n') => self.state.move_thread_down(),
            Key::Up | Key::Char('k') | Key::Ctrl('p') => self.state.move_thread_up(),
            Key::Char('o') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    if let Some(url) = bsky::get_url(post.author.handle.clone(), post.uri.clone()) {
                        if let Err(error) = webbrowser::open(&url) {
                            self.set_error(format!("Could not open thread post: {error}"));
                        }
                    }
                }
            }
            Key::Char('e') => {
                self.open_selected_embed(self.state.get_current_thread_post());
            }
            Key::Char('f') => {
                self.open_facet_viewer(self.state.get_current_thread_post());
            }
            Key::Char('a') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.dispatch(IoEvent::LoadProfile(post.author.did.clone().into()))
                        .await;
                }
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(self.state.get_current_thread_post());
            }
            Key::Char('X') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.start_quote_composer(&post);
                }
            }
            Key::Char('D') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.request_delete(&post);
                }
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.request_post_moderation(&post, key);
                }
            }
            Key::Char('H') => {
                let selected = self.state.get_current_thread_post();
                let did = self.state.get_did();
                let root = self
                    .state
                    .get_thread()
                    .into_iter()
                    .filter_map(|entry| entry.post().cloned())
                    .find(|post| post.author.did == did);
                if let (Some(root), Some(reply)) = (root, selected) {
                    if root.uri != reply.uri {
                        self.dispatch(IoEvent::Feature(FeatureEvent::ToggleHiddenReply {
                            root: Box::new(root),
                            reply: reply.uri,
                        }))
                        .await;
                    } else {
                        self.set_error("Select a reply beneath one of your posts".into());
                    }
                } else {
                    self.set_error("Hide reply is only available on your own thread".into());
                }
            }
            Key::Char('M') => {
                if let Some(root) = self
                    .state
                    .get_thread()
                    .into_iter()
                    .find_map(|entry| entry.post().cloned())
                {
                    let muted = config::AppConfig::load()
                        .map(|config| config.muted_threads.contains(&root.uri))
                        .unwrap_or(false);
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleThreadMute {
                        root: root.uri,
                        muted,
                    }))
                    .await;
                }
            }
            Key::Ctrl('d') => {
                if let Some(quote) = self.state.get_current_thread_post() {
                    if let Some((post, author)) = bsky::quoted_post(&quote) {
                        if author == self.state.get_did() {
                            self.dispatch(IoEvent::Feature(FeatureEvent::DetachQuote {
                                post,
                                quote: quote.uri,
                            }))
                            .await;
                        } else {
                            self.set_error("Only quotes of your own post can be detached".into());
                        }
                    } else {
                        self.set_error("The selected post is not a quote post".into());
                    }
                }
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.dispatch(IoEvent::LoadInteractions(
                        interaction_kind(key),
                        post.uri,
                        post.cid,
                    ))
                    .await;
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    fn open_selected_embed(
        &mut self,
        post: Option<atrium_api::app::bsky::feed::defs::PostViewData>,
    ) {
        let Some(url) = post.as_ref().and_then(bsky::post_embed_url) else {
            self.set_error("The selected post has no external link or video".to_owned());
            return;
        };
        if let Err(error) = webbrowser::open(&url) {
            self.set_error(format!("Could not open embedded content: {error}"));
        }
    }

    fn open_post_images(&mut self, post: Option<atrium_api::app::bsky::feed::defs::PostViewData>) {
        let moderation = self.state.moderation();
        let (urls, alt_texts) = post.map_or_else(
            || (Vec::new(), Vec::new()),
            |post| {
                (
                    bsky::post_attachment_fullsize_urls(&post, &moderation),
                    bsky::post_attachment_alt_texts(&post),
                )
            },
        );
        self.open_image_viewer(urls, alt_texts);
    }

    fn start_quote_composer(&mut self, post: &atrium_api::app::bsky::feed::defs::PostViewData) {
        self.state.set_input(Input::new(format!(
            "!quote {} | {}\n",
            post.uri,
            post.cid.as_ref()
        )));
        self.state.set_mode(state::Mode::Post);
    }

    fn request_post_moderation(
        &mut self,
        post: &atrium_api::app::bsky::feed::defs::PostViewData,
        key: Key,
    ) {
        if key == Key::Char('!') {
            self.open_report_prompt(feature_panel::ReportSubject::Record {
                uri: post.uri.clone(),
                cid: post.cid.clone(),
            });
            return;
        }
        self.pending_confirmation = Some(actor_moderation_action(&post.author, key));
    }

    fn request_profile_moderation(
        &mut self,
        profile: &atrium_api::app::bsky::actor::defs::ProfileViewDetailedData,
        key: Key,
    ) {
        if key == Key::Char('!') {
            self.open_report_prompt(feature_panel::ReportSubject::Account(profile.did.clone()));
            return;
        }
        let viewer = profile.viewer.as_ref();
        self.pending_confirmation = Some(match key {
            Key::Char('m') => ModerationAction::MuteActor {
                did: profile.did.clone(),
                muted: viewer.and_then(|viewer| viewer.muted).unwrap_or(false),
            },
            Key::Char('B') => ModerationAction::BlockActor {
                did: profile.did.clone(),
                blocking_uri: viewer.and_then(|viewer| viewer.blocking.clone()),
            },
            _ => ModerationAction::ReportActor(profile.did.clone()),
        });
    }

    fn open_report_prompt(&mut self, subject: feature_panel::ReportSubject) {
        let mut panel =
            feature_panel::FeaturePanel::loading(feature_panel::FeatureSection::Moderation);
        panel.title = "Report content".into();
        panel.prompt = Some(feature_panel::FeaturePrompt {
            label: "Report".into(),
            help: "spam|rude|sexual|violation|misleading|other | details".into(),
            action: feature_panel::FeaturePromptAction::Report { subject },
            input: Input::new("other | ".into()),
        });
        self.feature_panel = Some(panel);
    }

    fn request_delete(&mut self, post: &atrium_api::app::bsky::feed::defs::PostViewData) {
        if post.author.did != self.state.get_did() {
            self.set_error("Only your own posts can be deleted".to_owned());
            return;
        }
        self.pending_delete = Some(post.uri.clone());
    }

    async fn confirmation_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('y') | Key::Char('Y') | Key::Enter => {
                if let Some(uri) = self.pending_delete.take() {
                    self.dispatch(IoEvent::DeletePost(uri)).await;
                } else if let Some(action) = self.pending_confirmation.take() {
                    self.dispatch(IoEvent::Moderate(action)).await;
                }
            }
            Key::Char('n') | Key::Char('N') | Key::Esc | Key::Char('q') => {
                self.pending_confirmation = None;
                self.pending_delete = None;
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn confirmation_message(&self) -> Option<String> {
        self.pending_delete
            .as_ref()
            .map(|uri| format!("Delete your post {uri}? This cannot be undone."))
            .or_else(|| {
                self.pending_confirmation
                    .as_ref()
                    .map(ModerationAction::confirmation)
            })
    }

    async fn search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Enter => {
                let query = self.state.get_input().value().to_string();
                if !query.is_empty() {
                    self.state.set_search_query(Some(query.clone()));
                    self.dispatch(IoEvent::Search(SearchEvent::Load(query)))
                        .await;
                    self.state.set_mode(state::Mode::Normal);
                    self.state.set_tab(Tab::Search);
                    self.state.set_input(Input::default());
                }
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

    async fn user_search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
            }
            Key::Enter => {
                let query = self.state.get_input().value().trim().to_owned();
                if !query.is_empty() {
                    self.dispatch(IoEvent::SearchUsers(query)).await;
                }
            }
            Key::Left | Key::Ctrl('b') => self.state.move_input_cursor_prev(),
            Key::Right | Key::Ctrl('f') => self.state.move_input_cursor_next(),
            Key::Ctrl('a') => self.state.move_input_cursor_start(),
            Key::Ctrl('e') => self.state.move_input_cursor_end(),
            Key::Char(c) => self.state.insert_input(InputRequest::InsertChar(c)),
            Key::Backspace | Key::Ctrl('h') => self.state.remove_input_prev(),
            _ => {}
        }
        AppReturn::Continue
    }

    async fn feed_search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
            }
            Key::Enter => {
                let query = self.state.get_input().value().trim().to_owned();
                if !query.is_empty() {
                    self.dispatch(IoEvent::SearchFeeds(query)).await;
                    self.state.set_mode(state::Mode::Normal);
                    self.state.set_input(Input::default());
                }
            }
            Key::Left | Key::Ctrl('b') => self.state.move_input_cursor_prev(),
            Key::Right | Key::Ctrl('f') => self.state.move_input_cursor_next(),
            Key::Ctrl('a') => self.state.move_input_cursor_start(),
            Key::Ctrl('e') => self.state.move_input_cursor_end(),
            Key::Char(c) => self.state.insert_input(InputRequest::InsertChar(c)),
            Key::Backspace | Key::Ctrl('h') => self.state.remove_input_prev(),
            _ => {}
        }
        AppReturn::Continue
    }

    async fn post_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Ctrl('s') => {
                self.dispatch(IoEvent::SendPost).await;
                AppReturn::Continue
            }
            Key::Ctrl('v') => {
                self.preview_composer_link().await;
                AppReturn::Continue
            }
            Key::Enter => {
                self.state.insert_input(InputRequest::InsertChar('\n'));
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
            Key::Ctrl('s') => {
                if self.state.get_tab() == Tab::Search {
                    self.dispatch(IoEvent::SearchReply).await;
                } else {
                    self.dispatch(IoEvent::Reply).await;
                }
                AppReturn::Continue
            }
            Key::Ctrl('v') => {
                self.preview_composer_link().await;
                AppReturn::Continue
            }
            Key::Enter => {
                self.state.insert_input(InputRequest::InsertChar('\n'));
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
            Key::Esc | Key::Char('?') | Key::F1 => {
                self.close_help();
                AppReturn::Continue
            }
            Key::Char('j') | Key::Down => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some((selected + 1).min(ui::HELP_ROW_COUNT - 1)));
                AppReturn::Continue
            }
            Key::Char('k') | Key::Up => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some(selected.saturating_sub(1)));
                AppReturn::Continue
            }
            Key::PageDown => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some((selected + 10).min(ui::HELP_ROW_COUNT - 1)));
                AppReturn::Continue
            }
            Key::PageUp => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some(selected.saturating_sub(10)));
                AppReturn::Continue
            }
            Key::Home => {
                self.help_table_state.select(Some(0));
                AppReturn::Continue
            }
            Key::End => {
                self.help_table_state.select(Some(ui::HELP_ROW_COUNT - 1));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    fn image_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.image_viewer = None,
            Key::Char('h') | Key::Left => {
                if let Some(viewer) = &mut self.image_viewer {
                    viewer.previous();
                }
            }
            Key::Char('l') | Key::Right => {
                if let Some(viewer) = &mut self.image_viewer {
                    viewer.next();
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    fn facet_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.facet_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.facet_viewer {
                    viewer.previous();
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.facet_viewer {
                    viewer.next();
                }
            }
            Key::Enter | Key::Char('o') => {
                let url = self
                    .facet_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.facets.get(viewer.index))
                    .map(|facet| facet.url.clone());
                if let Some(url) = url {
                    if let Err(error) = webbrowser::open(&url) {
                        self.set_error(format!("Could not open facet: {error}"));
                    } else {
                        self.facet_viewer = None;
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    fn open_facet_viewer(&mut self, post: Option<atrium_api::app::bsky::feed::defs::PostViewData>) {
        let facets = post.as_ref().map_or_else(Vec::new, bsky::post_facets);
        self.facet_viewer = FacetViewer::new(facets);
        if self.facet_viewer.is_none() {
            self.set_error("The selected post has no URL, mention, or hashtag".to_owned());
        }
    }

    pub fn current_facet_viewer(&self) -> Option<(Vec<bsky::PostFacet>, usize)> {
        let viewer = self.facet_viewer.as_ref()?;
        Some((viewer.facets.clone(), viewer.index))
    }

    async fn interaction_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.interaction_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.interaction_viewer {
                    viewer.index = viewer.index.saturating_sub(1);
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.interaction_viewer {
                    if viewer.index + 1 < viewer.items.len() {
                        viewer.index += 1;
                    }
                }
            }
            Key::Enter | Key::Char('o') => {
                let selected = self
                    .interaction_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned();
                if let Some(actor) = selected.as_ref().and_then(|item| item.actor.clone()) {
                    self.dispatch(IoEvent::LoadProfile(actor)).await;
                } else if let Some(url) = selected.map(|item| item.url) {
                    if let Err(error) = webbrowser::open(&url) {
                        self.set_error(format!("Could not open interaction: {error}"));
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    async fn feature_panel_action(&mut self, key: Key) -> AppReturn {
        if self
            .feature_panel
            .as_ref()
            .and_then(|panel| panel.prompt.as_ref())
            .is_some()
        {
            return self.feature_prompt_action(key).await;
        }

        let section = match key {
            Key::Char('1') => Some(feature_panel::FeatureSection::Lists),
            Key::Char('2') => Some(feature_panel::FeatureSection::StarterPacks),
            Key::Char('3') => Some(feature_panel::FeatureSection::Discovery),
            Key::Char('4') => Some(feature_panel::FeatureSection::DirectMessages),
            Key::Char('5') => Some(feature_panel::FeatureSection::Moderation),
            Key::Char('6') => Some(feature_panel::FeatureSection::Settings),
            _ => None,
        };
        if let Some(section) = section {
            self.feature_panel = Some(feature_panel::FeaturePanel::loading(section));
            self.dispatch(IoEvent::Feature(FeatureEvent::Load(section)))
                .await;
            return AppReturn::Continue;
        }

        let selected = self
            .feature_panel
            .as_ref()
            .and_then(feature_panel::FeaturePanel::selected_row)
            .cloned();
        match key {
            Key::Esc => {
                let parent = self
                    .feature_panel
                    .as_mut()
                    .and_then(|panel| panel.parent.take())
                    .map(|parent| *parent);
                if let Some(parent) = parent {
                    self.feature_panel = Some(parent);
                } else {
                    self.feature_panel = None;
                }
            }
            Key::Char('k') | Key::Up => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.previous();
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.next();
                }
            }
            Key::Enter | Key::Char('o') => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List { uri, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenList(uri)))
                        .await
                }
                Some(feature_panel::FeatureTarget::StarterPack { uri, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenStarterPack(uri)))
                        .await
                }
                Some(feature_panel::FeatureTarget::Conversation { id, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenConversation(id)))
                        .await
                }
                Some(feature_panel::FeatureTarget::Labeler(did)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenLabeler(did)))
                        .await
                }
                Some(feature_panel::FeatureTarget::Actor(actor))
                | Some(feature_panel::FeatureTarget::ListMember { actor, .. }) => {
                    self.feature_panel = None;
                    self.dispatch(IoEvent::LoadProfile(actor)).await;
                }
                Some(feature_panel::FeatureTarget::Topic(topic)) => {
                    self.feature_panel = None;
                    self.state.set_search_query(Some(topic.clone()));
                    self.state.set_tab(Tab::Search);
                    self.dispatch(IoEvent::Search(SearchEvent::Load(topic)))
                        .await;
                }
                Some(feature_panel::FeatureTarget::Account(identifier)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::SwitchAccount(identifier)))
                        .await;
                }
                Some(feature_panel::FeatureTarget::Setting(setting)) => {
                    self.open_feature_prompt(
                        format!("Set {:?}", setting),
                        "Enter the new value".into(),
                        feature_panel::FeaturePromptAction::EditSetting(setting),
                        String::new(),
                    );
                }
                _ => {}
            },
            Key::Char('n') => {
                let section = self.feature_panel.as_ref().map(|panel| panel.section);
                match section {
                    Some(feature_panel::FeatureSection::Lists) => self.open_feature_prompt(
                        "Create list".into(),
                        "curation|moderation | name | description".into(),
                        feature_panel::FeaturePromptAction::CreateList,
                        "curation | ".into(),
                    ),
                    Some(feature_panel::FeatureSection::StarterPacks) => self.open_feature_prompt(
                        "Create Starter Pack".into(),
                        "name | description | list AT-URI".into(),
                        feature_panel::FeaturePromptAction::CreateStarterPack,
                        String::new(),
                    ),
                    Some(feature_panel::FeatureSection::DirectMessages) => self
                        .open_feature_prompt(
                            "New conversation".into(),
                            "Handle or DID".into(),
                            feature_panel::FeaturePromptAction::NewConversation,
                            String::new(),
                        ),
                    Some(feature_panel::FeatureSection::Moderation) => self.open_feature_prompt(
                        "Mute word".into(),
                        "Word or phrase".into(),
                        feature_panel::FeaturePromptAction::AddMutedWord,
                        String::new(),
                    ),
                    Some(feature_panel::FeatureSection::Settings) => self.open_feature_prompt(
                        "Add account".into(),
                        "identifier | service URL (save credentials separately)".into(),
                        feature_panel::FeaturePromptAction::AddAccount,
                        String::new(),
                    ),
                    _ => {}
                }
            }
            Key::Char('a') => {
                if let Some(feature_panel::FeatureTarget::List {
                    uri, owned: true, ..
                }) = selected.map(|row| row.target)
                {
                    self.open_feature_prompt(
                        "Add list member".into(),
                        "Handle or DID".into(),
                        feature_panel::FeaturePromptAction::AddListMember { list_uri: uri },
                        String::new(),
                    );
                }
            }
            Key::Char('e') => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List {
                    uri,
                    purpose,
                    owned: true,
                    ..
                }) => self.open_feature_prompt(
                    "Edit list".into(),
                    "name | description".into(),
                    feature_panel::FeaturePromptAction::EditList { uri, purpose },
                    String::new(),
                ),
                Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) => self.open_feature_prompt(
                    "Edit Starter Pack".into(),
                    "name | description | list AT-URI".into(),
                    feature_panel::FeaturePromptAction::EditStarterPack { uri },
                    String::new(),
                ),
                Some(feature_panel::FeatureTarget::LabelSetting { labeler, label }) => self
                    .open_feature_prompt(
                        format!("Label visibility · {label}"),
                        "ignore / warn / hide".into(),
                        feature_panel::FeaturePromptAction::SetLabelVisibility { labeler, label },
                        "warn".into(),
                    ),
                _ => {}
            },
            Key::Char('x') | Key::Delete => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List {
                    uri, owned: true, ..
                })
                | Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(uri)))
                        .await
                }
                Some(feature_panel::FeatureTarget::ListMember { item_uri, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(item_uri)))
                        .await
                }
                Some(feature_panel::FeatureTarget::MutedWord(word)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::RemoveMutedWord(word)))
                        .await
                }
                Some(feature_panel::FeatureTarget::MutedThread(root)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleThreadMute {
                        root,
                        muted: true,
                    }))
                    .await
                }
                _ => {}
            },
            Key::Char('s') => {
                if let Some(feature_panel::FeatureTarget::List {
                    uri,
                    purpose,
                    muted,
                    ..
                }) = selected.map(|row| row.target)
                {
                    if purpose == atrium_api::app::bsky::graph::defs::MODLIST {
                        self.dispatch(IoEvent::Feature(FeatureEvent::ToggleModerationList {
                            uri,
                            muted,
                        }))
                        .await;
                    } else {
                        self.dispatch(IoEvent::Feature(FeatureEvent::SaveList(uri)))
                            .await;
                    }
                }
            }
            Key::Char('f') => {
                if let Some(row) = selected {
                    if let feature_panel::FeatureTarget::List { uri, purpose, .. } = row.target {
                        if purpose == atrium_api::app::bsky::graph::defs::CURATELIST {
                            self.feature_panel = None;
                            self.dispatch(IoEvent::Feature(FeatureEvent::UseListFeed {
                                uri,
                                name: row.title,
                            }))
                            .await;
                        }
                    }
                }
            }
            Key::Char('l') => {
                if let Some(feature_panel::FeatureTarget::Labeler(did)) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleLabeler(did)))
                        .await;
                }
            }
            Key::Char('L') => {
                if self
                    .feature_panel
                    .as_ref()
                    .is_some_and(|panel| panel.section == feature_panel::FeatureSection::Moderation)
                {
                    self.open_feature_prompt(
                        "Subscribe to labeler".into(),
                        "Labeler DID".into(),
                        feature_panel::FeaturePromptAction::AddLabeler,
                        String::new(),
                    );
                }
            }
            Key::Char('J') => {
                if self.feature_panel.as_ref().is_some_and(|panel| {
                    panel.section == feature_panel::FeatureSection::StarterPacks
                }) {
                    let actors = self
                        .feature_panel
                        .as_ref()
                        .map(|panel| {
                            panel
                                .rows
                                .iter()
                                .filter_map(|row| match &row.target {
                                    feature_panel::FeatureTarget::Actor(actor) => {
                                        Some(actor.clone())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if actors.is_empty() {
                        self.set_error("Open a Starter Pack before joining it".into());
                    } else {
                        self.dispatch(IoEvent::Feature(FeatureEvent::JoinStarterPack(actors)))
                            .await;
                    }
                }
            }
            Key::Char(' ') => {
                if let Some(feature_panel::FeatureTarget::Conversation { id, muted, .. }) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleConversationMute {
                        convo_id: id,
                        muted,
                    }))
                    .await;
                }
            }
            Key::Char('w') => {
                let convo_id = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Conversation { id, .. }) => Some(id),
                    Some(feature_panel::FeatureTarget::Message { convo_id, .. }) => Some(convo_id),
                    _ => None,
                }
                .or_else(|| {
                    self.feature_panel.as_ref().and_then(|panel| {
                        panel
                            .title
                            .strip_prefix("Conversation · ")
                            .map(str::to_owned)
                    })
                });
                if let Some(convo_id) = convo_id {
                    self.open_feature_prompt(
                        "Send message".into(),
                        "Text message (1000 characters maximum)".into(),
                        feature_panel::FeaturePromptAction::SendMessage { convo_id },
                        String::new(),
                    );
                }
            }
            Key::Char('r') => {
                let subject = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Actor(AtIdentifier::Did(did)))
                    | Some(feature_panel::FeatureTarget::ListMember {
                        actor: AtIdentifier::Did(did),
                        ..
                    }) => Some(feature_panel::ReportSubject::Account(did)),
                    Some(feature_panel::FeatureTarget::Message {
                        convo_id,
                        id,
                        sender,
                    }) => Some(feature_panel::ReportSubject::Conversation {
                        convo_id,
                        message_id: Some(format!("{id}@{}", sender.as_str())),
                        sender,
                    }),
                    Some(feature_panel::FeatureTarget::List { uri, cid, .. })
                    | Some(feature_panel::FeatureTarget::StarterPack { uri, cid, .. }) => {
                        Some(feature_panel::ReportSubject::Record { uri, cid })
                    }
                    Some(feature_panel::FeatureTarget::Conversation { id, members, .. }) => members
                        .into_iter()
                        .next()
                        .map(|sender| feature_panel::ReportSubject::Conversation {
                            convo_id: id,
                            message_id: None,
                            sender,
                        }),
                    _ => None,
                };
                if let Some(subject) = subject {
                    self.open_feature_prompt(
                        "Report".into(),
                        "spam|rude|sexual|violation|misleading|other | details".into(),
                        feature_panel::FeaturePromptAction::Report { subject },
                        "other | ".into(),
                    );
                }
            }
            Key::Char('b') => {
                let did = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Actor(AtIdentifier::Did(did)))
                    | Some(feature_panel::FeatureTarget::ListMember {
                        actor: AtIdentifier::Did(did),
                        ..
                    }) => Some(did),
                    Some(feature_panel::FeatureTarget::Conversation { members, .. }) => {
                        members.into_iter().next()
                    }
                    Some(feature_panel::FeatureTarget::Message { sender, .. }) => Some(sender),
                    _ => None,
                };
                if let Some(did) = did {
                    self.pending_confirmation = Some(ModerationAction::BlockActor {
                        did,
                        blocking_uri: None,
                    });
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    async fn feature_prompt_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.prompt = None;
                }
            }
            Key::Enter => {
                let submitted = self.feature_panel.as_mut().and_then(|panel| {
                    panel
                        .prompt
                        .take()
                        .map(|prompt| (prompt.action, prompt.input.value().trim().to_owned()))
                });
                if let Some((action, value)) = submitted {
                    self.dispatch(IoEvent::Feature(FeatureEvent::Submit(action, value)))
                        .await;
                }
            }
            Key::Left | Key::Ctrl('b') => self.with_feature_input(InputRequest::GoToPrevChar),
            Key::Right | Key::Ctrl('f') => self.with_feature_input(InputRequest::GoToNextChar),
            Key::Ctrl('a') => self.with_feature_input(InputRequest::GoToStart),
            Key::Ctrl('e') => self.with_feature_input(InputRequest::GoToEnd),
            Key::Backspace | Key::Ctrl('h') => {
                self.with_feature_input(InputRequest::DeletePrevChar)
            }
            Key::Char(character) => {
                self.with_feature_input(InputRequest::InsertChar(character));
            }
            _ => {}
        }
        AppReturn::Continue
    }

    fn with_feature_input(&mut self, request: InputRequest) {
        if let Some(prompt) = self
            .feature_panel
            .as_mut()
            .and_then(|panel| panel.prompt.as_mut())
        {
            prompt.input.handle(request);
        }
    }

    fn open_feature_prompt(
        &mut self,
        label: String,
        help: String,
        action: feature_panel::FeaturePromptAction,
        initial: String,
    ) {
        if let Some(panel) = self.feature_panel.as_mut() {
            panel.prompt = Some(feature_panel::FeaturePrompt {
                label,
                help,
                action,
                input: Input::new(initial),
            });
        }
    }

    pub fn set_feature_rows(
        &mut self,
        title: String,
        rows: Vec<feature_panel::FeatureRow>,
        child: bool,
    ) {
        if let Some(panel) = self.feature_panel.as_mut() {
            if child {
                *panel = panel.child(title, rows);
            } else {
                panel.replace(title, rows);
            }
        }
    }

    pub fn feature_panel(&self) -> Option<&feature_panel::FeaturePanel> {
        self.feature_panel.as_ref()
    }

    async fn feed_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.feed_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.feed_viewer {
                    viewer.index = viewer.index.saturating_sub(1);
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.feed_viewer {
                    if viewer.index + 1 < viewer.items.len() {
                        viewer.index += 1;
                    }
                }
            }
            Key::Enter => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    self.feed_viewer = None;
                    self.dispatch(IoEvent::SelectFeed(feed)).await;
                }
            }
            Key::Char('s') => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    if matches!(feed.kind, feed::FeedKind::Custom(_)) {
                        self.dispatch(IoEvent::ToggleSavedFeed(feed)).await;
                    }
                }
            }
            Key::Char('!') => {
                let feed = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned();
                if let Some(feed::FeedDescriptor {
                    kind: feed::FeedKind::Custom(uri),
                    ..
                }) = feed
                {
                    self.feed_viewer = None;
                    self.open_report_prompt(feature_panel::ReportSubject::Feed(uri));
                }
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::FeedSearch);
                self.state.set_input(Input::default());
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn set_feed_catalog(&mut self, catalog: Vec<feed::FeedDescriptor>, open: bool) {
        self.feed_catalog = catalog.clone();
        if open {
            self.feed_viewer = Some(FeedViewer {
                items: catalog,
                index: 0,
            });
        }
    }

    pub fn set_feed_search_results(&mut self, results: Vec<feed::FeedDescriptor>) {
        self.feed_viewer = Some(FeedViewer {
            items: results,
            index: 0,
        });
    }

    pub fn current_feed_viewer(&self) -> Option<(Vec<feed::FeedDescriptor>, usize)> {
        let viewer = self.feed_viewer.as_ref()?;
        Some((viewer.items.clone(), viewer.index))
    }

    async fn preview_composer_link(&mut self) {
        match composer::parse_drafts(self.state.get_input().value())
            .ok()
            .and_then(|drafts| drafts.into_iter().find_map(|draft| draft.link))
        {
            Some(url) => self.dispatch(IoEvent::PreviewLink(url)).await,
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

    pub async fn update_on_tick(&mut self) -> AppReturn {
        self.images.poll();
        AppReturn::Continue
    }

    pub fn configure_images(&mut self, picker: ratatui_image::picker::Picker) {
        self.images.configure(picker);
    }

    pub fn queue_images<I>(&mut self, urls: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.images.queue(urls);
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

    pub async fn dispatch(&mut self, action: IoEvent) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        if self.io_tx.send(action).await.is_err() {
            self.is_loading = false;
            self.error = Some("Internal error: background worker is unavailable".to_string());
        };
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
            "red" => ratatui::style::Color::Red,
            "green" => ratatui::style::Color::Green,
            "yellow" => ratatui::style::Color::Yellow,
            "magenta" => ratatui::style::Color::Magenta,
            "cyan" => ratatui::style::Color::Cyan,
            "white" => ratatui::style::Color::White,
            _ => ratatui::style::Color::Blue,
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

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }
}

fn interaction_kind(key: Key) -> InteractionKind {
    match key {
        Key::Char('L') => InteractionKind::Likes,
        Key::Char('R') => InteractionKind::Reposts,
        _ => InteractionKind::Quotes,
    }
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
        Key::F1 => binding == "f1",
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

    #[tokio::test]
    async fn help_navigation_moves_and_stays_within_bounds() {
        let (io_tx, _io_rx) = tokio::sync::mpsc::channel(1);
        let mut app = App::new(io_tx);

        app.help_action(Key::Up).await;
        assert_eq!(app.help_table_state.selected(), Some(0));

        app.help_action(Key::PageDown).await;
        assert_eq!(app.help_table_state.selected(), Some(10));

        app.help_action(Key::End).await;
        app.help_action(Key::Down).await;
        assert_eq!(
            app.help_table_state.selected(),
            Some(ui::HELP_ROW_COUNT - 1)
        );

        app.help_action(Key::Home).await;
        assert_eq!(app.help_table_state.selected(), Some(0));
    }
}
