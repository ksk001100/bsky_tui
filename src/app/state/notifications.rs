//! notifications state operations.

use super::*;

impl AppState {
    pub fn set_notification_posts(&mut self, posts: HashMap<String, PostViewData>) {
        if let Self::Initialized {
            notification_posts, ..
        } = self
        {
            *notification_posts = posts;
        }
    }

    pub fn notification_post(&self, uri: &str) -> Option<PostViewData> {
        if let Self::Initialized {
            notification_posts, ..
        } = self
        {
            notification_posts.get(uri).cloned()
        } else {
            None
        }
    }

    pub fn set_notifications(&mut self, n: Option<Vec<Notification>>) {
        if let Self::Initialized {
            notifications,
            notifications_list_position,
            notifications_list_state,
            ..
        } = self
        {
            let is_empty = n.as_ref().is_none_or(Vec::is_empty);
            *notifications = n;
            *notifications_list_position = 0;
            notifications_list_state.select((!is_empty).then_some(0));
        }
    }

    pub fn get_notifications(&self) -> Option<Vec<Notification>> {
        if let Self::Initialized { notifications, .. } = self {
            notifications.clone()
        } else {
            None
        }
    }

    pub fn get_current_notification(&self) -> Option<Notification> {
        if let Self::Initialized {
            notifications,
            notifications_list_position,
            ..
        } = self
        {
            let filters = self.notification_filters();
            notifications.as_ref().and_then(|items| {
                notifications::groups(items, filters)
                    .get(*notifications_list_position)
                    .map(|group| group.primary().clone())
            })
        } else {
            None
        }
    }

    pub fn get_current_notification_post(&self) -> Option<PostViewData> {
        let notification = self.get_current_notification()?;
        let uri = notifications::post_uri(&notification)?;
        self.notification_post(uri)
    }

    pub fn notification_filters(&self) -> NotificationFilters {
        if let Self::Initialized {
            notification_filters,
            ..
        } = self
        {
            *notification_filters
        } else {
            NotificationFilters::default()
        }
    }

    pub fn notification_groups(&self) -> Vec<NotificationGroup> {
        self.get_notifications()
            .map(|items| notifications::groups(&items, self.notification_filters()))
            .unwrap_or_default()
    }

    pub fn cycle_notification_reason_filter(&mut self) {
        if let Self::Initialized {
            notification_filters,
            notifications,
            notifications_list_position,
            notifications_list_state,
            ..
        } = self
        {
            notification_filters.reason = notification_filters.reason.next();
            *notifications_list_position = 0;
            let has_items = notifications.as_ref().is_some_and(|items| {
                !notifications::groups(items, *notification_filters).is_empty()
            });
            notifications_list_state.select(has_items.then_some(0));
        }
    }

    pub fn cycle_notification_sender_filter(&mut self) {
        if let Self::Initialized {
            notification_filters,
            notifications,
            notifications_list_position,
            notifications_list_state,
            ..
        } = self
        {
            notification_filters.sender = notification_filters.sender.next();
            *notifications_list_position = 0;
            let has_items = notifications.as_ref().is_some_and(|items| {
                !notifications::groups(items, *notification_filters).is_empty()
            });
            notifications_list_state.select(has_items.then_some(0));
        }
    }

    pub fn cycle_notification_read_filter(&mut self) {
        if let Self::Initialized {
            notification_filters,
            notifications,
            notifications_list_position,
            notifications_list_state,
            ..
        } = self
        {
            notification_filters.read = notification_filters.read.next();
            *notifications_list_position = 0;
            let has_items = notifications.as_ref().is_some_and(|items| {
                !notifications::groups(items, *notification_filters).is_empty()
            });
            notifications_list_state.select(has_items.then_some(0));
        }
    }

    pub fn get_notifications_current_cursor_index(&self) -> usize {
        if let Self::Initialized {
            notifications_current_cursor_index,
            ..
        } = self
        {
            *notifications_current_cursor_index
        } else {
            0
        }
    }

    pub fn set_notifications_current_cursor_index(&mut self, index: usize) {
        if let Self::Initialized {
            notifications_current_cursor_index,
            ..
        } = self
        {
            *notifications_current_cursor_index = index;
        }
    }

    pub fn get_notification_cursors(&self) -> Vec<Option<String>> {
        if let Self::Initialized {
            notification_cursors,
            ..
        } = self
        {
            notification_cursors.clone()
        } else {
            Vec::new()
        }
    }

    pub fn set_notification_cursors(&mut self, mut value: Vec<Option<String>>) {
        value.truncate(MAX_CURSOR_HISTORY);
        if let Self::Initialized {
            notification_cursors,
            ..
        } = self
        {
            *notification_cursors = value;
        }
    }
}
