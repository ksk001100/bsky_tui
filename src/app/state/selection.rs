//! selection state operations.

use super::*;

impl AppState {
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
            if *tl_list_position + 1 < feeds.len() {
                *tl_list_position += 1;
                tl_list_state.select(Some(*tl_list_position));
            }
        }
    }

    pub fn move_tl_scroll_by(&mut self, delta: isize) {
        if let Self::Initialized {
            tl_list_position,
            tl_list_state,
            timeline,
            ..
        } = self
        {
            select_by(
                tl_list_position,
                tl_list_state,
                timeline.as_ref().map_or(0, Vec::len),
                delta,
            );
        }
    }

    pub fn move_tl_half_up(&mut self) {
        self.move_tl_scroll_by(-(HALF_PAGE_ITEMS as isize));
    }
    pub fn move_tl_half_down(&mut self) {
        self.move_tl_scroll_by(HALF_PAGE_ITEMS as isize);
    }
    pub fn move_tl_scroll_bottom(&mut self) {
        self.move_tl_scroll_by(isize::MAX);
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
            notification_filters,
            ..
        } = self
        {
            let len = notifications::groups(notifications, *notification_filters).len();
            if *notifications_list_position + 1 < len {
                *notifications_list_position += 1;
                notifications_list_state.select(Some(*notifications_list_position));
            }
        }
    }

    pub fn move_notifications_scroll_by(&mut self, delta: isize) {
        if let Self::Initialized {
            notifications_list_position,
            notifications_list_state,
            notifications,
            notification_filters,
            ..
        } = self
        {
            let len = notifications.as_ref().map_or(0, |items| {
                notifications::groups(items, *notification_filters).len()
            });
            select_by(
                notifications_list_position,
                notifications_list_state,
                len,
                delta,
            );
        }
    }

    pub fn move_notifications_top(&mut self) {
        self.move_notifications_scroll_by(isize::MIN);
    }
    pub fn move_notifications_bottom(&mut self) {
        self.move_notifications_scroll_by(isize::MAX);
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
            if *search_list_position + 1 < results.len() {
                *search_list_position += 1;
                search_list_state.select(Some(*search_list_position));
            }
        }
    }

    pub fn move_search_scroll_by(&mut self, delta: isize) {
        if let Self::Initialized {
            search_list_position,
            search_list_state,
            search_results,
            ..
        } = self
        {
            select_by(
                search_list_position,
                search_list_state,
                search_results.as_ref().map_or(0, Vec::len),
                delta,
            );
        }
    }

    pub fn move_search_top(&mut self) {
        self.move_search_scroll_by(isize::MIN);
    }
    pub fn move_search_bottom(&mut self) {
        self.move_search_scroll_by(isize::MAX);
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
}
