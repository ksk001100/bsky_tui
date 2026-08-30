//! Navigation state operations.

use super::*;

impl AppState {
    pub fn set_mode(&mut self, mode: Mode) {
        if let Some(model) = self.model_mut() {
            model.navigation.mode = mode;
        }
    }

    pub fn get_mode(&self) -> Mode {
        self.model()
            .map_or(Mode::Normal, |model| model.navigation.mode)
    }

    pub fn is_normal_mode(&self) -> bool {
        self.has_mode(Mode::Normal)
    }
    pub fn is_post_mode(&self) -> bool {
        self.has_mode(Mode::Post)
    }
    pub fn is_reply_mode(&self) -> bool {
        self.has_mode(Mode::Reply)
    }
    pub fn is_help_mode(&self) -> bool {
        self.has_mode(Mode::Help)
    }
    pub fn is_search_mode(&self) -> bool {
        self.has_mode(Mode::Search)
    }
    pub fn is_user_search_mode(&self) -> bool {
        self.has_mode(Mode::UserSearch)
    }
    pub fn is_feed_search_mode(&self) -> bool {
        self.has_mode(Mode::FeedSearch)
    }

    fn has_mode(&self, mode: Mode) -> bool {
        self.model()
            .is_some_and(|model| model.navigation.mode == mode)
    }

    pub fn get_tl_list_state(&self) -> ListState {
        self.model()
            .map_or_else(ListState::default, |model| model.home.selection.list)
    }

    pub fn get_notifications_list_state(&self) -> ListState {
        self.model().map_or_else(ListState::default, |model| {
            model.notifications.selection.list
        })
    }

    pub fn get_current_feed(&self) -> Option<FeedViewPost> {
        let model = self.model()?;
        model
            .home
            .timeline
            .as_ref()
            .and_then(|items| items.get(model.home.selection.position))
            .cloned()
    }

    pub fn get_tab(&self) -> Tab {
        self.model().map_or(Tab::Home, |model| model.navigation.tab)
    }

    pub fn set_tab(&mut self, tab: Tab) {
        if let Some(model) = self.model_mut() {
            model.navigation.tab = tab;
        }
    }

    pub fn set_next_tab(&mut self) {
        if let Some(model) = self.model_mut() {
            model.navigation.tab = match model.navigation.tab {
                Tab::Home => Tab::Notifications,
                Tab::Notifications => Tab::Messages,
                Tab::Messages => Tab::Search,
                Tab::Search => Tab::Home,
            };
        }
    }
}
