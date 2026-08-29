pub mod auth;
pub mod composer;
pub mod config;
pub mod feed;
pub mod images;
pub mod moderation;
pub mod notifications;
pub mod profile;
pub mod state;
pub mod thread;
pub mod ui;

use atrium_api::types::string::{Did, Handle};
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
        InteractionKind, IoEvent, ModerationAction, NotificationEvent, SearchEvent, TimelineEvent,
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
                "↑/↓ select   i image   o browser   a author   Esc back   ? help"
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
        self.pending_confirmation = Some(match key {
            Key::Char('!') => ModerationAction::ReportPost {
                uri: post.uri.clone(),
                cid: post.cid.clone(),
            },
            _ => actor_moderation_action(&post.author, key),
        });
    }

    fn request_profile_moderation(
        &mut self,
        profile: &atrium_api::app::bsky::actor::defs::ProfileViewDetailedData,
        key: Key,
    ) {
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
    ) {
        self.state = AppState::initialized(agent, handle, did, moderation);
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
