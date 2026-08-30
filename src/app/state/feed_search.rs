//! Home feed and explore state operations.

use super::*;

impl AppState {
    pub fn set_timeline(&mut self, timeline: Option<Vec<FeedViewPost>>) {
        if let Some(model) = self.model_mut() {
            let is_empty = timeline.as_ref().is_none_or(Vec::is_empty);
            model.home.timeline = timeline;
            model.home.selection.position = 0;
            model.home.selection.list.select((!is_empty).then_some(0));
        }
    }

    pub fn set_timeline_preserving_position(
        &mut self,
        timeline: Option<Vec<FeedViewPost>>,
        position: usize,
    ) {
        if let Some(model) = self.model_mut() {
            let length = timeline.as_ref().map_or(0, Vec::len);
            model.home.timeline = timeline;
            model.home.selection.position = position.min(length.saturating_sub(1));
            model
                .home
                .selection
                .list
                .select((length > 0).then_some(model.home.selection.position));
        }
    }

    pub fn get_active_feed(&self) -> FeedDescriptor {
        self.model()
            .map_or_else(FeedDescriptor::following, |model| {
                model.home.active_feed.clone()
            })
    }

    pub fn activate_feed(&mut self, descriptor: FeedDescriptor) {
        let Some(model) = self.model_mut() else {
            return;
        };
        let home = &mut model.home;
        home.feed_snapshots.insert(
            home.active_feed.id.clone(),
            FeedSnapshot {
                timeline: home.timeline.clone(),
                position: home.selection.position,
                page: home.pagination.current,
                cursors: home.pagination.cursors.clone(),
                new_count: home
                    .feed_snapshots
                    .get(&home.active_feed.id)
                    .map_or(0, |snapshot| snapshot.new_count),
            },
        );
        while home.feed_snapshots.len() > MAX_FEED_SNAPSHOTS {
            let removable = home
                .feed_snapshots
                .keys()
                .filter(|id| *id != &home.active_feed.id && *id != &descriptor.id)
                .min()
                .cloned();
            let Some(removable) = removable else { break };
            home.feed_snapshots.remove(&removable);
        }
        let snapshot = home
            .feed_snapshots
            .get(&descriptor.id)
            .cloned()
            .unwrap_or_default();
        home.active_feed = descriptor;
        home.timeline = snapshot.timeline;
        home.selection.position = snapshot.position;
        home.pagination.current = snapshot.page;
        home.pagination.cursors = snapshot.cursors;
        home.selection.list.select(
            home.timeline
                .as_ref()
                .is_some_and(|items| !items.is_empty())
                .then_some(home.selection.position),
        );
    }

    pub fn set_active_feed_new_count(&mut self, count: usize) {
        if let Some(model) = self.model_mut() {
            model
                .home
                .feed_snapshots
                .entry(model.home.active_feed.id.clone())
                .or_default()
                .new_count = count;
        }
    }

    pub fn get_active_feed_new_count(&self) -> usize {
        self.model()
            .and_then(|model| model.home.feed_snapshots.get(&model.home.active_feed.id))
            .map_or(0, |snapshot| snapshot.new_count)
    }

    pub fn get_timeline(&self) -> Option<Vec<FeedViewPost>> {
        self.model().and_then(|model| model.home.timeline.clone())
    }

    pub(crate) fn has_timeline(&self) -> bool {
        self.model()
            .is_some_and(|model| model.home.timeline.is_some())
    }

    pub fn set_search_results(&mut self, results: Option<Vec<PostViewData>>) {
        if let Some(model) = self.model_mut() {
            let is_empty = results.as_ref().is_none_or(Vec::is_empty);
            model.explore.results = results;
            model.explore.selection.position = 0;
            model
                .explore
                .selection
                .list
                .select((!is_empty).then_some(0));
        }
    }

    pub fn get_search_results(&self) -> Option<Vec<PostViewData>> {
        self.model().and_then(|model| model.explore.results.clone())
    }

    pub fn get_search_list_state(&self) -> ListState {
        self.model()
            .map_or_else(ListState::default, |model| model.explore.selection.list)
    }

    pub fn move_search_scroll_top(&mut self) {
        if let Some(model) = self.model_mut() {
            model.explore.selection.position = 0;
            model.explore.selection.list.select(Some(0));
        }
    }

    pub fn get_current_search_result(&self) -> Option<PostViewData> {
        let model = self.model()?;
        model
            .explore
            .results
            .as_ref()
            .and_then(|items| items.get(model.explore.selection.position))
            .cloned()
    }
}
