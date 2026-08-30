//! navigation state operations.

use super::*;

impl AppState {
    pub fn set_mode(&mut self, mode: Mode) {
        if let Self::Initialized { mode: m, .. } = self {
            *m = mode;
        }
    }

    pub fn get_mode(&self) -> Mode {
        if let Self::Initialized { mode, .. } = self {
            *mode
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

    pub fn is_user_search_mode(&self) -> bool {
        matches!(
            self,
            Self::Initialized {
                mode: Mode::UserSearch,
                ..
            }
        )
    }

    pub fn is_feed_search_mode(&self) -> bool {
        matches!(
            self,
            Self::Initialized {
                mode: Mode::FeedSearch,
                ..
            }
        )
    }

    pub fn get_tl_list_state(&self) -> ListState {
        if let Self::Initialized { tl_list_state, .. } = self {
            *tl_list_state
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
            *notifications_list_state
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
            *tab
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
                Tab::Notifications => Tab::Messages,
                Tab::Messages => Tab::Search,
                Tab::Search => Tab::Home,
            }
        }
    }
}
