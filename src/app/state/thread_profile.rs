//! Thread and profile state operations.

use super::*;

impl AppState {
    pub fn set_thread(&mut self, entries: Vec<ThreadEntry>) {
        if let Some(model) = self.model_mut() {
            let selected = entries
                .iter()
                .position(|entry| matches!(entry, ThreadEntry::Post { target: true, .. }))
                .unwrap_or(0);
            model.thread.entries = Some(entries);
            model.thread.selection.position = selected;
            model.thread.selection.list.select(Some(selected));
            model.navigation.mode = Mode::Thread;
        }
    }

    pub fn get_thread(&self) -> Vec<ThreadEntry> {
        self.model()
            .and_then(|model| model.thread.entries.clone())
            .unwrap_or_default()
    }

    pub fn get_thread_list_state(&self) -> ListState {
        self.model()
            .map_or_else(ListState::default, |model| model.thread.selection.list)
    }

    pub fn get_thread_list_position(&self) -> usize {
        self.model()
            .map_or(0, |model| model.thread.selection.position)
    }

    pub fn get_current_thread_post(&self) -> Option<PostViewData> {
        let model = self.model()?;
        model
            .thread
            .entries
            .as_ref()?
            .get(model.thread.selection.position)
            .and_then(ThreadEntry::post)
            .cloned()
    }

    pub fn move_thread_up(&mut self) {
        if let Some(model) = self.model_mut() {
            let selection = &mut model.thread.selection;
            selection.position = selection.position.saturating_sub(1);
            selection.list.select(Some(selection.position));
        }
    }
    pub fn move_thread_down(&mut self) {
        if let Some(model) = self.model_mut() {
            let thread = &mut model.thread;
            let length = thread.entries.as_ref().map_or(0, Vec::len);
            if thread.selection.position + 1 < length {
                thread.selection.position += 1;
                thread
                    .selection
                    .list
                    .select(Some(thread.selection.position));
            }
        }
    }

    pub fn move_thread_by(&mut self, delta: isize) {
        if let Some(model) = self.model_mut() {
            let length = model.thread.entries.as_ref().map_or(0, Vec::len);
            select_by(
                &mut model.thread.selection.position,
                &mut model.thread.selection.list,
                length,
                delta,
            );
        }
    }

    pub fn move_thread_top(&mut self) {
        self.move_thread_by(isize::MIN);
    }
    pub fn move_thread_bottom(&mut self) {
        self.move_thread_by(isize::MAX);
    }

    pub fn close_thread(&mut self) {
        if let Some(model) = self.model_mut() {
            model.navigation.mode = Mode::Normal;
            model.thread.entries = None;
        }
    }

    pub fn is_thread_mode(&self) -> bool {
        self.model()
            .is_some_and(|model| model.navigation.mode == Mode::Thread)
    }

    pub fn open_profile(&mut self, profile: ProfileState) {
        if let Some(model) = self.model_mut() {
            model.profile = Some(profile);
            model.navigation.mode = Mode::Profile;
        }
    }

    pub fn close_profile(&mut self) {
        if let Some(model) = self.model_mut() {
            model.navigation.mode = Mode::Normal;
            model.profile = None;
        }
    }

    pub fn is_profile_mode(&self) -> bool {
        self.model()
            .is_some_and(|model| model.navigation.mode == Mode::Profile)
    }

    pub fn get_profile(&self) -> Option<ProfileState> {
        self.model().and_then(|model| model.profile.clone())
    }

    pub(crate) fn get_profile_identity(&self) -> Option<(Did, ProfileSection)> {
        self.model()
            .and_then(|model| model.profile.as_ref())
            .map(|profile| (profile.details.did.clone(), profile.section))
    }

    pub fn set_profile_content(&mut self, section: ProfileSection, content: ProfileContent) {
        if let Some(profile) = self.model_mut().and_then(|model| model.profile.as_mut()) {
            profile.set_content(section, content);
        }
    }

    pub fn move_profile_up(&mut self) {
        self.move_profile_by(-1);
    }
    pub fn move_profile_down(&mut self) {
        self.move_profile_by(1);
    }

    pub fn move_profile_by(&mut self, delta: isize) {
        if let Some(profile) = self.model_mut().and_then(|model| model.profile.as_mut()) {
            select_by(
                &mut profile.position,
                &mut profile.list_state,
                profile.content.len(),
                delta,
            );
        }
    }

    pub fn move_profile_top(&mut self) {
        self.move_profile_by(isize::MIN);
    }
    pub fn move_profile_bottom(&mut self) {
        self.move_profile_by(isize::MAX);
    }

    pub fn get_current_profile_post(&self) -> Option<FeedViewPost> {
        let profile = self.get_profile()?;
        match profile.content {
            ProfileContent::Posts(posts) => posts.get(profile.position).cloned(),
            ProfileContent::Items(_) => None,
        }
    }

    pub fn get_current_profile_item(&self) -> Option<crate::app::profile::ProfileListItem> {
        let profile = self.get_profile()?;
        match profile.content {
            ProfileContent::Items(items) => items.get(profile.position).cloned(),
            ProfileContent::Posts(_) => None,
        }
    }
}
