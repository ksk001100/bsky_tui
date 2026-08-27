pub mod auth;
pub mod config;
pub mod images;
pub mod moderation;
pub mod state;
pub mod thread;
pub mod ui;

use atrium_api::types::string::{Did, Handle};
use bsky_sdk::BskyAgent;
use tui_input::{Input, InputRequest};

use self::images::ImageCache;
use self::state::AppState;
use crate::{
    app::state::Tab,
    bsky,
    inputs::key::Key,
    io::{InteractionKind, IoEvent, NotificationEvent, SearchEvent, TimelineEvent},
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

        if self.image_viewer.is_some() {
            return self.image_viewer_action(key);
        }
        if self.facet_viewer.is_some() {
            return self.facet_viewer_action(key);
        }
        if self.interaction_viewer.is_some() {
            return self.interaction_viewer_action(key);
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
        }
    }

    async fn timeline_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('r') => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload))
                    .await;
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
            Key::Char('b') => {
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
            Key::Char('t') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::LoadThread(feed.post.uri.clone()))
                        .await;
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
            Key::Enter => {
                let moderation = self.state.moderation();
                let (urls, alt_texts) = self.state.get_current_feed().map_or_else(
                    || (Vec::new(), Vec::new()),
                    |feed| {
                        (
                            bsky::post_attachment_fullsize_urls(&feed.post, &moderation),
                            bsky::post_attachment_alt_texts(&feed.post),
                        )
                    },
                );
                self.open_image_viewer(urls, alt_texts);
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
            Key::Char('h') | Key::Left => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Prev))
                    .await;
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Next))
                    .await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn notifications_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('r') => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Reload))
                    .await;
                AppReturn::Continue
            }
            Key::Char('?') => {
                self.state.set_mode(state::Mode::Help);
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
                self.state.move_notifications_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_notifications_scroll_up();
                AppReturn::Continue
            }
            Key::Enter | Key::Char('b') => {
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
            Key::Char('h') | Key::Left => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Prev))
                    .await;
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Next))
                    .await;
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    async fn search_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('r') => {
                self.dispatch(IoEvent::Search(SearchEvent::Reload)).await;
                AppReturn::Continue
            }
            Key::Char('N') => {
                self.state.set_tab(Tab::Search);
                self.state.set_mode(state::Mode::Reply);
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
            Key::Char('?') => {
                self.state.set_mode(state::Mode::Help);
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
            Key::Char('b') => {
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
            Key::Char('t') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadThread(post.uri.clone())).await;
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
            Key::Enter => {
                let moderation = self.state.moderation();
                let (urls, alt_texts) = self.state.get_current_search_result().map_or_else(
                    || (Vec::new(), Vec::new()),
                    |post| {
                        (
                            bsky::post_attachment_fullsize_urls(&post, &moderation),
                            bsky::post_attachment_alt_texts(&post),
                        )
                    },
                );
                self.open_image_viewer(urls, alt_texts);
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
            Key::Char('h') | Key::Left => {
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
            Key::Char('l') | Key::Right => {
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

    async fn thread_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc => self.state.close_thread(),
            Key::Down | Key::Char('j') | Key::Ctrl('n') => self.state.move_thread_down(),
            Key::Up | Key::Char('k') | Key::Ctrl('p') => self.state.move_thread_up(),
            Key::Char('b') => {
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
            Key::Char('q') | Key::Esc | Key::Char('?') => {
                self.state.set_mode(state::Mode::Normal);
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    fn image_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc => self.image_viewer = None,
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
            Key::Char('q') | Key::Esc => self.facet_viewer = None,
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
            Key::Enter | Key::Char('b') => {
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

    fn interaction_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('q') | Key::Esc => self.interaction_viewer = None,
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
            Key::Enter | Key::Char('b') => {
                let url = self
                    .interaction_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .map(|item| item.url.clone());
                if let Some(url) = url {
                    if let Err(error) = webbrowser::open(&url) {
                        self.set_error(format!("Could not open interaction: {error}"));
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn set_interactions(&mut self, kind: InteractionKind, items: Vec<bsky::InteractionItem>) {
        self.interaction_viewer = Some(InteractionViewer {
            kind,
            items,
            index: 0,
        });
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
}
