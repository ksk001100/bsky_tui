//! Notification state operations.

use super::*;

fn reset_filtered_selection(state: &mut NotificationState) {
    state.selection.position = 0;
    let has_items = state
        .items
        .as_ref()
        .is_some_and(|items| !notifications::groups(items, state.filters).is_empty());
    state.selection.list.select(has_items.then_some(0));
}

impl AppState {
    pub fn set_notification_posts(&mut self, posts: HashMap<String, PostViewData>) {
        if let Some(model) = self.model_mut() {
            model.notifications.posts = posts;
        }
    }

    pub fn notification_post(&self, uri: &str) -> Option<PostViewData> {
        self.model()
            .and_then(|model| model.notifications.posts.get(uri).cloned())
    }

    pub fn set_notifications(&mut self, items: Option<Vec<Notification>>) {
        if let Some(model) = self.model_mut() {
            let is_empty = items.as_ref().is_none_or(Vec::is_empty);
            model.notifications.items = items;
            model.notifications.selection.position = 0;
            model
                .notifications
                .selection
                .list
                .select((!is_empty).then_some(0));
        }
    }

    pub fn get_notifications(&self) -> Option<Vec<Notification>> {
        self.model()
            .and_then(|model| model.notifications.items.clone())
    }

    pub fn get_current_notification(&self) -> Option<Notification> {
        let state = &self.model()?.notifications;
        state.items.as_ref().and_then(|items| {
            notifications::groups(items, state.filters)
                .get(state.selection.position)
                .map(|group| group.primary().clone())
        })
    }

    pub fn get_current_notification_post(&self) -> Option<PostViewData> {
        let notification = self.get_current_notification()?;
        self.notification_post(notifications::post_uri(&notification)?)
    }

    pub fn notification_filters(&self) -> NotificationFilters {
        self.model()
            .map_or_else(NotificationFilters::default, |model| {
                model.notifications.filters
            })
    }

    pub fn notification_groups(&self) -> Vec<NotificationGroup> {
        self.model()
            .and_then(|model| {
                model
                    .notifications
                    .items
                    .as_ref()
                    .map(|items| notifications::groups(items, model.notifications.filters))
            })
            .unwrap_or_default()
    }

    pub fn cycle_notification_reason_filter(&mut self) {
        if let Some(model) = self.model_mut() {
            model.notifications.filters.reason = model.notifications.filters.reason.next();
            reset_filtered_selection(&mut model.notifications);
        }
    }

    pub fn cycle_notification_sender_filter(&mut self) {
        if let Some(model) = self.model_mut() {
            model.notifications.filters.sender = model.notifications.filters.sender.next();
            reset_filtered_selection(&mut model.notifications);
        }
    }

    pub fn cycle_notification_read_filter(&mut self) {
        if let Some(model) = self.model_mut() {
            model.notifications.filters.read = model.notifications.filters.read.next();
            reset_filtered_selection(&mut model.notifications);
        }
    }

    pub fn get_notifications_current_cursor_index(&self) -> usize {
        self.model()
            .map_or(0, |model| model.notifications.pagination.current)
    }

    pub fn set_notifications_current_cursor_index(&mut self, index: usize) {
        if let Some(model) = self.model_mut() {
            model.notifications.pagination.current = index;
        }
    }

    pub fn get_notification_cursors(&self) -> Vec<Option<String>> {
        self.model().map_or_else(Vec::new, |model| {
            model.notifications.pagination.cursors.clone()
        })
    }

    pub fn set_notification_cursors(&mut self, mut cursors: Vec<Option<String>>) {
        cursors.truncate(MAX_CURSOR_HISTORY);
        if let Some(model) = self.model_mut() {
            model.notifications.pagination.cursors = cursors;
        }
    }
}
