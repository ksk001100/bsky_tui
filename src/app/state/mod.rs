use std::{collections::HashMap, fmt, sync::Arc};

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

use crate::app::feed::{FeedDescriptor, FeedSnapshot};
use crate::app::moderation::ModerationPrefs;
use crate::app::notifications::{self, NotificationFilters, NotificationGroup};
use crate::app::profile::{ProfileContent, ProfileSection, ProfileState};
use crate::app::thread::ThreadEntry;

const MAX_CURSOR_HISTORY: usize = 20;
const HALF_PAGE_ITEMS: usize = 5;
const MAX_FEED_SNAPSHOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Post,
    Reply,
    Help,
    Search,
    UserSearch,
    Thread,
    Profile,
    FeedSearch,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Mode::Normal => "Normal",
            Mode::Post => "Post",
            Mode::Reply => "Reply",
            Mode::Help => "Help",
            Mode::Search => "Search",
            Mode::UserSearch => "User Search",
            Mode::Thread => "Thread",
            Mode::Profile => "Profile",
            Mode::FeedSearch => "Feed Search",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Notifications,
    Messages,
    Search,
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Tab::Home => "Home",
            Tab::Notifications => "Notifications",
            Tab::Messages => "Messages",
            Tab::Search => "Explore",
        };
        write!(f, "{}", str)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Default)]
pub enum AppState {
    #[default]
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
        notifications_current_cursor_index: usize,
        notification_cursors: Vec<Option<String>>,
        notification_filters: NotificationFilters,
        notification_posts: HashMap<String, PostViewData>,
        search_list_state: ListState,
        search_list_position: usize,
        handle: Handle,
        did: Did,
        mode: Mode,
        tab: Tab,
        tl_current_cursor_index: usize,
        cursors: Vec<Option<String>>,
        search_current_cursor_index: usize,
        search_cursors: Vec<Option<String>>,
        search_query: Option<String>,
        moderation: ModerationPrefs,
        thread: Option<Vec<ThreadEntry>>,
        thread_list_state: ListState,
        thread_list_position: usize,
        profile: Option<ProfileState>,
        active_feed: FeedDescriptor,
        feed_snapshots: HashMap<String, FeedSnapshot>,
    },
}

impl AppState {
    pub fn initialized(
        agent: BskyAgent,
        handle: Handle,
        did: Did,
        moderation: ModerationPrefs,
    ) -> Self {
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
            notifications_current_cursor_index: 0,
            notification_cursors: vec![None],
            notification_filters: NotificationFilters::default(),
            notification_posts: HashMap::new(),
            search_list_state: ListState::default().with_selected(Some(0)),
            search_list_position: 0,
            handle,
            did,
            mode: Mode::Normal,
            tab: Tab::Home,
            tl_current_cursor_index: 0,
            cursors: vec![None],
            search_current_cursor_index: 0,
            search_cursors: vec![None],
            search_query: None,
            moderation,
            thread: None,
            thread_list_state: ListState::default(),
            thread_list_position: 0,
            profile: None,
            active_feed: FeedDescriptor::following(),
            feed_snapshots: HashMap::new(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn get_handle(&self) -> Option<Handle> {
        if let Self::Initialized { handle, .. } = self {
            Some(handle.clone())
        } else {
            None
        }
    }

    pub fn get_agent(&self) -> Option<Arc<BskyAgent>> {
        if let Self::Initialized { agent, .. } = self {
            Some(agent.clone())
        } else {
            None
        }
    }

    pub fn moderation(&self) -> ModerationPrefs {
        if let Self::Initialized { moderation, .. } = self {
            moderation.clone()
        } else {
            ModerationPrefs::default()
        }
    }
}

mod feed_search;
mod input;
mod navigation;
#[path = "notifications.rs"]
mod notification_state;
mod pagination;
mod selection;
mod thread_profile;

fn select_by(position: &mut usize, state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        *position = 0;
        state.select(None);
        return;
    }
    *position = if delta == isize::MIN {
        0
    } else if delta == isize::MAX {
        len - 1
    } else if delta < 0 {
        position.saturating_sub(delta.unsigned_abs())
    } else {
        position.saturating_add(delta as usize).min(len - 1)
    };
    state.select(Some(*position));
}
