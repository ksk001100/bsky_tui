//! Pagination state operations.

use super::*;

fn current_cursor(pagination: &PaginationState) -> Option<String> {
    pagination
        .cursors
        .get(pagination.current)
        .cloned()
        .flatten()
}

fn next_cursor(pagination: &PaginationState) -> Option<String> {
    pagination
        .cursors
        .get(pagination.current + 1)
        .cloned()
        .flatten()
}

fn previous_cursor(pagination: &PaginationState) -> Option<String> {
    pagination
        .current
        .checked_sub(1)
        .and_then(|index| pagination.cursors.get(index))
        .cloned()
        .flatten()
}

fn set_cursors(pagination: &mut PaginationState, mut cursors: Vec<Option<String>>) {
    cursors.truncate(MAX_CURSOR_HISTORY);
    pagination.cursors = cursors;
}

impl AppState {
    pub fn get_tl_current_cursor_index(&self) -> usize {
        self.model()
            .map_or(0, |model| model.home.pagination.current)
    }

    pub fn set_tl_current_cursor_index(&mut self, index: usize) {
        if let Some(model) = self.model_mut() {
            model.home.pagination.current = index;
        }
    }

    pub fn get_cursors(&self) -> Vec<Option<String>> {
        self.model()
            .map_or_else(Vec::new, |model| model.home.pagination.cursors.clone())
    }

    pub fn set_cursors(&mut self, cursors: Vec<Option<String>>) {
        if let Some(model) = self.model_mut() {
            set_cursors(&mut model.home.pagination, cursors);
        }
    }

    pub fn get_current_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| current_cursor(&model.home.pagination))
    }

    pub fn get_next_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| next_cursor(&model.home.pagination))
    }

    pub fn get_prev_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| previous_cursor(&model.home.pagination))
    }

    pub fn get_search_current_cursor_index(&self) -> usize {
        self.model()
            .map_or(0, |model| model.explore.pagination.current)
    }

    pub fn set_search_current_cursor_index(&mut self, index: usize) {
        if let Some(model) = self.model_mut() {
            model.explore.pagination.current = index;
        }
    }

    pub fn get_search_cursors(&self) -> Vec<Option<String>> {
        self.model()
            .map_or_else(Vec::new, |model| model.explore.pagination.cursors.clone())
    }

    pub fn set_search_cursors(&mut self, cursors: Vec<Option<String>>) {
        if let Some(model) = self.model_mut() {
            set_cursors(&mut model.explore.pagination, cursors);
        }
    }

    pub fn get_search_current_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| current_cursor(&model.explore.pagination))
    }

    pub fn get_search_next_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| next_cursor(&model.explore.pagination))
    }

    pub fn get_search_prev_cursor(&self) -> Option<String> {
        self.model()
            .and_then(|model| previous_cursor(&model.explore.pagination))
    }

    pub fn get_search_query(&self) -> Option<String> {
        self.model().and_then(|model| model.explore.query.clone())
    }

    pub fn set_search_query(&mut self, query: Option<String>) {
        if let Some(model) = self.model_mut() {
            model.explore.query = query;
        }
    }
}
