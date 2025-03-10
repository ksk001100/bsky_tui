use std::{fmt, sync::Arc};

use atrium_api::{
    app::bsky::{
        feed::defs::{FeedViewPost, PostViewData},
        notification::list_notifications::Notification,
    },
    types::string::{Did, Handle},
};
use bsky_sdk::BskyAgent;
use ratatui::widgets::ListState;
use tui_input::{Input, InputRequest};

use crate::app::config::AppConfig;

#[derive(Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Post,
    Reply,
    Help,
    Search,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Mode::Normal => "Normal",
            Mode::Post => "Post",
            Mode::Reply => "Reply",
            Mode::Help => "Help",
            Mode::Search => "Search",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Tab {
    Home,
    Notifications,
    Search,
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Tab::Home => "Home",
            Tab::Notifications => "Notifications",
            Tab::Search => "Search",
        };
        write!(f, "{}", str)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        agent: Arc<BskyAgent>,
        timeline: Option<Vec<FeedViewPost>>,
        notifications: Option<Vec<Notification>>,
        search_results: Option<Vec<PostViewData>>,
        input: Input,
        tl_list_state: ListState,
        tl_list_position: usize,
        notifications_list_state: ListState,
        notifications_list_position: usize,
        search_list_state: ListState,
        search_list_position: usize,
        handle: Handle,
        did: Did,
        mode: Mode,
        tab: Tab,
        config: Box<AppConfig>,
        tl_current_cursor_index: usize,
        cursors: Vec<Option<String>>,
        search_current_cursor_index: usize,
        search_cursors: Vec<Option<String>>,
        search_query: Option<String>,
        is_loading: bool,
    },
}

impl AppState {
    pub fn initialized(agent: BskyAgent, handle: Handle, did: Did, config: AppConfig) -> Self {
        let agent = Arc::new(agent);
        Self::Initialized {
            agent,
            timeline: None,
            notifications: None,
            search_results: None,
            input: Input::default(),
            tl_list_state: ListState::default().with_selected(Some(0)),
            tl_list_position: 0,
            notifications_list_state: ListState::default().with_selected(Some(0)),
            notifications_list_position: 0,
            search_list_state: ListState::default().with_selected(Some(0)),
            search_list_position: 0,
            handle,
            did,
            mode: Mode::Normal,
            tab: Tab::Home,
            config: Box::new(config),
            tl_current_cursor_index: 0,
            cursors: vec![None],
            search_current_cursor_index: 0,
            search_cursors: vec![None],
            search_query: None,
            is_loading: false,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn get_handle(&self) -> Handle {
        if let Self::Initialized { handle, .. } = self {
            handle.clone()
        } else {
            Handle::new("".to_string()).unwrap()
        }
    }

    pub fn get_agent(&self) -> Option<Arc<BskyAgent>> {
        if let Self::Initialized { agent, .. } = self {
            Some(agent.clone())
        } else {
            None
        }
    }

    pub fn get_did(&self) -> Did {
        if let Self::Initialized { did, .. } = self {
            did.clone()
        } else {
            Did::new("".to_string()).unwrap()
        }
    }

    pub fn get_input(&self) -> Input {
        if let Self::Initialized { input, .. } = self {
            input.clone()
        } else {
            Input::default()
        }
    }

    pub fn set_input(&mut self, i: Input) {
        if let Self::Initialized { input, .. } = self {
            *input = i;
        }
    }

    pub fn insert_input(&mut self, req: InputRequest) {
        if let Self::Initialized { input: i, .. } = self {
            i.handle(req);
        }
    }

    pub fn move_input_cursor_prev(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToPrevChar);
        }
    }

    pub fn move_input_cursor_next(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToNextChar);
        }
    }

    pub fn move_input_cursor_start(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToStart);
        }
    }

    pub fn move_input_cursor_end(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToEnd);
        }
    }

    pub fn remove_input_prev(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::DeletePrevChar);
        }
    }

    pub fn move_tl_scroll_top(&mut self) {
        if let Self::Initialized {
            tl_list_state,
            tl_list_position,
            ..
        } = self
        {
            *tl_list_position = 0;
            tl_list_state.select(Some(0));
        }
    }

    pub fn move_tl_scroll_up(&mut self) {
        if let Self::Initialized {
            tl_list_position,
            tl_list_state,
            ..
        } = self
        {
            if *tl_list_position > 0 {
                *tl_list_position -= 1;
                tl_list_state.select(Some(*tl_list_position));
            }
        }
    }

    pub fn move_tl_scroll_down(&mut self) {
        if let Self::Initialized {
            tl_list_position,
            tl_list_state,
            timeline: Some(feeds),
            ..
        } = self
        {
            if *tl_list_position < feeds.len() - 1 {
                *tl_list_position += 1;
                tl_list_state.select(Some(*tl_list_position));
            }
        }
    }

    pub fn get_tl_list_position(&self) -> usize {
        if let Self::Initialized {
            tl_list_position, ..
        } = self
        {
            *tl_list_position
        } else {
            0
        }
    }

    pub fn move_notifications_scroll_up(&mut self) {
        if let Self::Initialized {
            notifications_list_position,
            notifications_list_state,
            ..
        } = self
        {
            if *notifications_list_position > 0 {
                *notifications_list_position -= 1;
                notifications_list_state.select(Some(*notifications_list_position));
            }
        }
    }

    pub fn move_notifications_scroll_down(&mut self) {
        if let Self::Initialized {
            notifications_list_position,
            notifications_list_state,
            notifications: Some(notifications),
            ..
        } = self
        {
            if *notifications_list_position < notifications.len() - 1 {
                *notifications_list_position += 1;
                notifications_list_state.select(Some(*notifications_list_position));
            }
        }
    }

    pub fn get_notifications_list_position(&self) -> usize {
        if let Self::Initialized {
            notifications_list_position,
            ..
        } = self
        {
            *notifications_list_position
        } else {
            0
        }
    }

    pub fn move_search_scroll_up(&mut self) {
        if let Self::Initialized {
            search_list_position,
            search_list_state,
            ..
        } = self
        {
            if *search_list_position > 0 {
                *search_list_position -= 1;
                search_list_state.select(Some(*search_list_position));
            }
        }
    }

    pub fn move_search_scroll_down(&mut self) {
        if let Self::Initialized {
            search_list_position,
            search_list_state,
            search_results: Some(results),
            ..
        } = self
        {
            if *search_list_position < results.len() - 1 {
                *search_list_position += 1;
                search_list_state.select(Some(*search_list_position));
            }
        }
    }

    pub fn get_search_list_position(&self) -> usize {
        if let Self::Initialized {
            search_list_position,
            ..
        } = self
        {
            *search_list_position
        } else {
            0
        }
    }

    pub fn set_timeline(&mut self, f: Option<Vec<FeedViewPost>>) {
        if let Self::Initialized { timeline, .. } = self {
            *timeline = f;
        }
    }

    pub fn get_timeline(&self) -> Option<Vec<FeedViewPost>> {
        if let Self::Initialized { timeline, .. } = self {
            timeline.clone()
        } else {
            None
        }
    }

    pub fn set_search_results(&mut self, f: Option<Vec<PostViewData>>) {
        if let Self::Initialized { search_results, .. } = self {
            *search_results = f;
        }
    }

    pub fn get_search_results(&self) -> Option<Vec<PostViewData>> {
        if let Self::Initialized { search_results, .. } = self {
            search_results.clone()
        } else {
            None
        }
    }

    pub fn get_search_list_state(&self) -> ListState {
        if let Self::Initialized {
            search_list_state, ..
        } = self
        {
            search_list_state.clone()
        } else {
            ListState::default()
        }
    }

    pub fn move_search_scroll_top(&mut self) {
        if let Self::Initialized {
            search_list_state,
            search_list_position,
            ..
        } = self
        {
            *search_list_position = 0;
            search_list_state.select(Some(0));
        }
    }

    pub fn get_current_search_result(&self) -> Option<PostViewData> {
        if let Self::Initialized {
            search_results,
            search_list_position,
            ..
        } = self
        {
            search_results
                .clone()
                .and_then(|f| f.get(*search_list_position).cloned())
        } else {
            None
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if let Self::Initialized { mode: m, .. } = self {
            *m = mode;
        }
    }

    pub fn get_mode(&self) -> Mode {
        if let Self::Initialized { mode, .. } = self {
            mode.clone()
        } else {
            Mode::Normal
        }
    }

    pub fn is_normal_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Normal)
        } else {
            false
        }
    }

    pub fn is_post_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Post)
        } else {
            false
        }
    }

    pub fn is_reply_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Reply)
        } else {
            false
        }
    }

    pub fn is_help_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Help)
        } else {
            false
        }
    }

    pub fn is_search_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Search)
        } else {
            false
        }
    }

    pub fn get_tl_list_state(&self) -> ListState {
        if let Self::Initialized { tl_list_state, .. } = self {
            tl_list_state.clone()
        } else {
            ListState::default()
        }
    }

    pub fn get_notifications_list_state(&self) -> ListState {
        if let Self::Initialized {
            notifications_list_state,
            ..
        } = self
        {
            notifications_list_state.clone()
        } else {
            ListState::default()
        }
    }

    pub fn get_current_feed(&self) -> Option<FeedViewPost> {
        if let Self::Initialized {
            timeline,
            tl_list_position,
            ..
        } = self
        {
            timeline
                .clone()
                .and_then(|f| f.get(*tl_list_position).cloned())
        } else {
            None
        }
    }

    pub fn get_tab(&self) -> Tab {
        if let Self::Initialized { tab, .. } = self {
            tab.clone()
        } else {
            Tab::Home
        }
    }

    pub fn set_tab(&mut self, tab: Tab) {
        if let Self::Initialized { tab: t, .. } = self {
            *t = tab;
        }
    }

    pub fn set_next_tab(&mut self) {
        if let Self::Initialized { tab, .. } = self {
            *tab = match tab {
                Tab::Home => Tab::Notifications,
                Tab::Notifications => Tab::Search,
                Tab::Search => Tab::Home,
            }
        }
    }

    pub fn set_notifications(&mut self, n: Option<Vec<Notification>>) {
        if let Self::Initialized { notifications, .. } = self {
            *notifications = n;
        }
    }

    pub fn get_notifications(&self) -> Option<Vec<Notification>> {
        if let Self::Initialized { notifications, .. } = self {
            notifications.clone()
        } else {
            None
        }
    }

    pub fn get_tl_current_cursor_index(&self) -> usize {
        if let Self::Initialized {
            tl_current_cursor_index,
            ..
        } = self
        {
            *tl_current_cursor_index
        } else {
            0
        }
    }

    pub fn set_tl_current_cursor_index(&mut self, index: usize) {
        if let Self::Initialized {
            tl_current_cursor_index,
            ..
        } = self
        {
            *tl_current_cursor_index = index;
        }
    }

    pub fn get_cursors(&self) -> Vec<Option<String>> {
        if let Self::Initialized { cursors, .. } = self {
            cursors.clone()
        } else {
            Vec::new()
        }
    }

    pub fn set_cursors(&mut self, cursors: Vec<Option<String>>) {
        if let Self::Initialized { cursors: c, .. } = self {
            *c = cursors;
        }
    }

    pub fn get_current_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            tl_current_cursor_index,
            cursors,
            ..
        } = self
        {
            cursors.get(*tl_current_cursor_index).cloned().unwrap()
        } else {
            None
        }
    }

    pub fn get_next_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            tl_current_cursor_index,
            cursors,
            ..
        } = self
        {
            if *tl_current_cursor_index + 1 == cursors.len() {
                return None;
            }
            cursors.get(*tl_current_cursor_index + 1).cloned().unwrap()
        } else {
            None
        }
    }

    pub fn get_prev_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            tl_current_cursor_index,
            cursors,
            ..
        } = self
        {
            if *tl_current_cursor_index == 0 {
                return None;
            }
            cursors.get(*tl_current_cursor_index - 1).cloned().unwrap()
        } else {
            None
        }
    }

    pub fn set_loading(&mut self, is_loading: bool) {
        if let Self::Initialized { is_loading: l, .. } = self {
            *l = is_loading;
        }
    }

    pub fn is_loading(&self) -> bool {
        if let Self::Initialized { is_loading, .. } = self {
            *is_loading
        } else {
            false
        }
    }

    pub fn get_search_current_cursor_index(&self) -> usize {
        if let Self::Initialized {
            search_current_cursor_index,
            ..
        } = self
        {
            *search_current_cursor_index
        } else {
            0
        }
    }

    pub fn set_search_current_cursor_index(&mut self, index: usize) {
        if let Self::Initialized {
            search_current_cursor_index,
            ..
        } = self
        {
            *search_current_cursor_index = index;
        }
    }

    pub fn get_search_cursors(&self) -> Vec<Option<String>> {
        if let Self::Initialized { search_cursors, .. } = self {
            search_cursors.clone()
        } else {
            Vec::new()
        }
    }

    pub fn set_search_cursors(&mut self, cursors: Vec<Option<String>>) {
        if let Self::Initialized {
            search_cursors: c, ..
        } = self
        {
            *c = cursors;
        }
    }

    pub fn get_search_current_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            search_current_cursor_index,
            search_cursors,
            ..
        } = self
        {
            search_cursors
                .get(*search_current_cursor_index)
                .cloned()
                .unwrap()
        } else {
            None
        }
    }

    pub fn get_search_next_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            search_current_cursor_index,
            search_cursors,
            ..
        } = self
        {
            if *search_current_cursor_index + 1 == search_cursors.len() {
                return None;
            }
            search_cursors
                .get(*search_current_cursor_index + 1)
                .cloned()
                .unwrap()
        } else {
            None
        }
    }

    pub fn get_search_prev_cursor(&self) -> Option<String> {
        if let Self::Initialized {
            search_current_cursor_index,
            search_cursors,
            ..
        } = self
        {
            if *search_current_cursor_index == 0 {
                return None;
            }
            search_cursors
                .get(*search_current_cursor_index - 1)
                .cloned()
                .unwrap()
        } else {
            None
        }
    }

    pub fn get_search_query(&self) -> Option<String> {
        if let Self::Initialized { search_query, .. } = self {
            search_query.clone()
        } else {
            None
        }
    }

    pub fn set_search_query(&mut self, query: Option<String>) {
        if let Self::Initialized { search_query, .. } = self {
            *search_query = query;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init
    }
}
