//! feed search state operations.

use super::*;

impl AppState {
    pub fn set_timeline(&mut self, f: Option<Vec<FeedViewPost>>) {
        if let Self::Initialized {
            timeline,
            tl_list_position,
            tl_list_state,
            ..
        } = self
        {
            let is_empty = f.as_ref().is_none_or(Vec::is_empty);
            *timeline = f;
            *tl_list_position = 0;
            tl_list_state.select((!is_empty).then_some(0));
        }
    }

    pub fn set_timeline_preserving_position(
        &mut self,
        timeline_value: Option<Vec<FeedViewPost>>,
        position: usize,
    ) {
        if let Self::Initialized {
            timeline,
            tl_list_position,
            tl_list_state,
            ..
        } = self
        {
            let length = timeline_value.as_ref().map_or(0, Vec::len);
            *timeline = timeline_value;
            *tl_list_position = position.min(length.saturating_sub(1));
            tl_list_state.select((length > 0).then_some(*tl_list_position));
        }
    }

    pub fn get_active_feed(&self) -> FeedDescriptor {
        if let Self::Initialized { active_feed, .. } = self {
            active_feed.clone()
        } else {
            FeedDescriptor::following()
        }
    }

    pub fn activate_feed(&mut self, descriptor: FeedDescriptor) {
        if let Self::Initialized {
            timeline,
            tl_list_position,
            tl_list_state,
            tl_current_cursor_index,
            cursors,
            active_feed,
            feed_snapshots,
            ..
        } = self
        {
            feed_snapshots.insert(
                active_feed.id.clone(),
                FeedSnapshot {
                    timeline: timeline.clone(),
                    position: *tl_list_position,
                    page: *tl_current_cursor_index,
                    cursors: cursors.clone(),
                    new_count: feed_snapshots
                        .get(&active_feed.id)
                        .map_or(0, |snapshot| snapshot.new_count),
                },
            );
            while feed_snapshots.len() > MAX_FEED_SNAPSHOTS {
                let removable = feed_snapshots
                    .keys()
                    .filter(|id| *id != &active_feed.id && *id != &descriptor.id)
                    .min()
                    .cloned();
                let Some(removable) = removable else { break };
                feed_snapshots.remove(&removable);
            }
            let snapshot = feed_snapshots
                .get(&descriptor.id)
                .cloned()
                .unwrap_or_default();
            *active_feed = descriptor;
            *timeline = snapshot.timeline;
            *tl_list_position = snapshot.position;
            *tl_current_cursor_index = snapshot.page;
            *cursors = snapshot.cursors;
            tl_list_state.select(
                timeline
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
                    .then_some(*tl_list_position),
            );
        }
    }

    pub fn set_active_feed_new_count(&mut self, count: usize) {
        if let Self::Initialized {
            active_feed,
            feed_snapshots,
            ..
        } = self
        {
            feed_snapshots
                .entry(active_feed.id.clone())
                .or_default()
                .new_count = count;
        }
    }

    pub fn get_active_feed_new_count(&self) -> usize {
        if let Self::Initialized {
            active_feed,
            feed_snapshots,
            ..
        } = self
        {
            feed_snapshots
                .get(&active_feed.id)
                .map_or(0, |snapshot| snapshot.new_count)
        } else {
            0
        }
    }

    pub fn get_timeline(&self) -> Option<Vec<FeedViewPost>> {
        if let Self::Initialized { timeline, .. } = self {
            timeline.clone()
        } else {
            None
        }
    }

    pub fn set_search_results(&mut self, f: Option<Vec<PostViewData>>) {
        if let Self::Initialized {
            search_results,
            search_list_position,
            search_list_state,
            ..
        } = self
        {
            let is_empty = f.as_ref().is_none_or(Vec::is_empty);
            *search_results = f;
            *search_list_position = 0;
            search_list_state.select((!is_empty).then_some(0));
        }
    }

    pub fn get_search_results(&self) -> Option<Vec<PostViewData>> {
        if let Self::Initialized { search_results, .. } = self {
            search_results.clone()
        } else {
            None
        }
    }

    pub fn get_search_list_state(&self) -> ListState {
        if let Self::Initialized {
            search_list_state, ..
        } = self
        {
            *search_list_state
        } else {
            ListState::default()
        }
    }

    pub fn move_search_scroll_top(&mut self) {
        if let Self::Initialized {
            search_list_state,
            search_list_position,
            ..
        } = self
        {
            *search_list_position = 0;
            search_list_state.select(Some(0));
        }
    }

    pub fn get_current_search_result(&self) -> Option<PostViewData> {
        if let Self::Initialized {
            search_results,
            search_list_position,
            ..
        } = self
        {
            search_results
                .clone()
                .and_then(|f| f.get(*search_list_position).cloned())
        } else {
            None
        }
    }
}
