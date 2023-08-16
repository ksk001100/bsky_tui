use std::fmt;
use std::sync::Arc;

use atrium_api::app::bsky::{
    feed::defs::FeedViewPost, notification::list_notifications::Notification,
};
use ratatui::widgets::ListState;
use tui_input::{Input, InputRequest};

use crate::bsky;

#[derive(Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Post,
    Reply,
    Help,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Mode::Normal => "Normal",
            Mode::Post => "Post",
            Mode::Reply => "Reply",
            Mode::Help => "Help",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Tab {
    Timeline,
    Notifications,
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Tab::Timeline => "Timeline",
            Tab::Notifications => "Notifications",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        agent: Arc<bsky::Agent>,
        timeline: Option<Vec<FeedViewPost>>,
        notifications: Option<Vec<Notification>>,
        input: Input,
        tl_list_state: ListState,
        tl_list_position: usize,
        notifications_list_state: ListState,
        notifications_list_position: usize,
        handle: Option<String>,
        mode: Mode,
        tab: Tab,
    },
}

impl AppState {
    pub fn initialized(agent: bsky::Agent, handle: String) -> Self {
        let agent = Arc::new(agent);
        Self::Initialized {
            agent,
            timeline: None,
            notifications: None,
            input: Input::default(),
            tl_list_state: ListState::default().with_selected(Some(0)),
            tl_list_position: 0,
            notifications_list_state: ListState::default().with_selected(Some(0)),
            notifications_list_position: 0,
            handle: Some(handle),
            mode: Mode::Normal,
            tab: Tab::Timeline,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn get_handle(&self) -> Option<String> {
        if let Self::Initialized { handle, .. } = self {
            handle.clone()
        } else {
            None
        }
    }

    pub fn get_agent(&self) -> Option<Arc<bsky::Agent>> {
        if let Self::Initialized { agent, .. } = self {
            Some(agent.clone())
        } else {
            None
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
            Tab::Timeline
        }
    }

    pub fn set_next_tab(&mut self) {
        if let Self::Initialized { tab, .. } = self {
            *tab = match tab {
                Tab::Timeline => Tab::Notifications,
                Tab::Notifications => Tab::Timeline,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init
    }
}
