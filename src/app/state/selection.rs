//! List selection state operations.

use super::*;

impl AppState {
    pub fn move_tl_scroll_top(&mut self) {
        if let Some(model) = self.model_mut() {
            model.home.selection.position = 0;
            model.home.selection.list.select(Some(0));
        }
    }

    pub fn move_tl_scroll_up(&mut self) {
        if let Some(model) = self.model_mut() {
            let selection = &mut model.home.selection;
            selection.position = selection.position.saturating_sub(1);
            selection.list.select(Some(selection.position));
        }
    }
    pub fn move_tl_scroll_down(&mut self) {
        if let Some(model) = self.model_mut() {
            let home = &mut model.home;
            if home
                .timeline
                .as_ref()
                .is_some_and(|items| home.selection.position + 1 < items.len())
            {
                home.selection.position += 1;
                home.selection.list.select(Some(home.selection.position));
            }
        }
    }

    pub fn move_tl_scroll_by(&mut self, delta: isize) {
        if let Some(model) = self.model_mut() {
            let length = model.home.timeline.as_ref().map_or(0, Vec::len);
            select_by(
                &mut model.home.selection.position,
                &mut model.home.selection.list,
                length,
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
        self.model()
            .map_or(0, |model| model.home.selection.position)
    }

    pub fn move_notifications_scroll_up(&mut self) {
        if let Some(model) = self.model_mut() {
            let selection = &mut model.notifications.selection;
            selection.position = selection.position.saturating_sub(1);
            selection.list.select(Some(selection.position));
        }
    }
    pub fn move_notifications_scroll_down(&mut self) {
        if let Some(model) = self.model_mut() {
            let state = &mut model.notifications;
            let length = state
                .items
                .as_ref()
                .map_or(0, |items| notifications::groups(items, state.filters).len());
            if state.selection.position + 1 < length {
                state.selection.position += 1;
                state.selection.list.select(Some(state.selection.position));
            }
        }
    }

    pub fn move_notifications_scroll_by(&mut self, delta: isize) {
        if let Some(model) = self.model_mut() {
            let state = &mut model.notifications;
            let length = state
                .items
                .as_ref()
                .map_or(0, |items| notifications::groups(items, state.filters).len());
            select_by(
                &mut state.selection.position,
                &mut state.selection.list,
                length,
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
        self.model()
            .map_or(0, |model| model.notifications.selection.position)
    }

    pub fn move_search_scroll_up(&mut self) {
        if let Some(model) = self.model_mut() {
            let selection = &mut model.explore.selection;
            selection.position = selection.position.saturating_sub(1);
            selection.list.select(Some(selection.position));
        }
    }
    pub fn move_search_scroll_down(&mut self) {
        if let Some(model) = self.model_mut() {
            let explore = &mut model.explore;
            if explore
                .results
                .as_ref()
                .is_some_and(|items| explore.selection.position + 1 < items.len())
            {
                explore.selection.position += 1;
                explore
                    .selection
                    .list
                    .select(Some(explore.selection.position));
            }
        }
    }

    pub fn move_search_scroll_by(&mut self, delta: isize) {
        if let Some(model) = self.model_mut() {
            let length = model.explore.results.as_ref().map_or(0, Vec::len);
            select_by(
                &mut model.explore.selection.position,
                &mut model.explore.selection.list,
                length,
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
        self.model()
            .map_or(0, |model| model.explore.selection.position)
    }
}
