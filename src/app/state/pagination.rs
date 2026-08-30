//! pagination state operations.

use super::*;

impl AppState {
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

    pub fn set_cursors(&mut self, mut cursors: Vec<Option<String>>) {
        cursors.truncate(MAX_CURSOR_HISTORY);
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
            cursors.get(*tl_current_cursor_index).cloned().flatten()
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
            cursors.get(*tl_current_cursor_index + 1).cloned().flatten()
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
            cursors.get(*tl_current_cursor_index - 1).cloned().flatten()
        } else {
            None
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

    pub fn set_search_cursors(&mut self, mut cursors: Vec<Option<String>>) {
        cursors.truncate(MAX_CURSOR_HISTORY);
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
                .flatten()
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
            search_cursors
                .get(*search_current_cursor_index + 1)
                .cloned()
                .flatten()
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
                .flatten()
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
