//! thread profile state operations.

use super::*;

impl AppState {
    pub fn set_thread(&mut self, entries: Vec<ThreadEntry>) {
        if let Self::Initialized {
            thread,
            thread_list_state,
            thread_list_position,
            mode,
            ..
        } = self
        {
            let selected = entries
                .iter()
                .position(|entry| matches!(entry, ThreadEntry::Post { target: true, .. }))
                .unwrap_or(0);
            *thread = Some(entries);
            *thread_list_position = selected;
            thread_list_state.select(Some(selected));
            *mode = Mode::Thread;
        }
    }

    pub fn get_thread(&self) -> Vec<ThreadEntry> {
        if let Self::Initialized { thread, .. } = self {
            thread.clone().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn get_thread_list_state(&self) -> ListState {
        if let Self::Initialized {
            thread_list_state, ..
        } = self
        {
            *thread_list_state
        } else {
            ListState::default()
        }
    }

    pub fn get_thread_list_position(&self) -> usize {
        if let Self::Initialized {
            thread_list_position,
            ..
        } = self
        {
            *thread_list_position
        } else {
            0
        }
    }

    pub fn get_current_thread_post(&self) -> Option<PostViewData> {
        self.get_thread()
            .get(self.get_thread_list_position())
            .and_then(ThreadEntry::post)
            .cloned()
    }

    pub fn move_thread_up(&mut self) {
        if let Self::Initialized {
            thread_list_position,
            thread_list_state,
            ..
        } = self
        {
            *thread_list_position = thread_list_position.saturating_sub(1);
            thread_list_state.select(Some(*thread_list_position));
        }
    }

    pub fn move_thread_down(&mut self) {
        if let Self::Initialized {
            thread,
            thread_list_position,
            thread_list_state,
            ..
        } = self
        {
            let length = thread.as_ref().map_or(0, Vec::len);
            if *thread_list_position + 1 < length {
                *thread_list_position += 1;
                thread_list_state.select(Some(*thread_list_position));
            }
        }
    }

    pub fn move_thread_by(&mut self, delta: isize) {
        if let Self::Initialized {
            thread,
            thread_list_position,
            thread_list_state,
            ..
        } = self
        {
            select_by(
                thread_list_position,
                thread_list_state,
                thread.as_ref().map_or(0, Vec::len),
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
        if let Self::Initialized { mode, thread, .. } = self {
            *mode = Mode::Normal;
            *thread = None;
        }
    }

    pub fn is_thread_mode(&self) -> bool {
        matches!(
            self,
            Self::Initialized {
                mode: Mode::Thread,
                ..
            }
        )
    }

    pub fn open_profile(&mut self, profile_state: ProfileState) {
        if let Self::Initialized { mode, profile, .. } = self {
            *profile = Some(profile_state);
            *mode = Mode::Profile;
        }
    }

    pub fn close_profile(&mut self) {
        if let Self::Initialized { mode, profile, .. } = self {
            *mode = Mode::Normal;
            *profile = None;
        }
    }

    pub fn is_profile_mode(&self) -> bool {
        matches!(
            self,
            Self::Initialized {
                mode: Mode::Profile,
                ..
            }
        )
    }

    pub fn get_profile(&self) -> Option<ProfileState> {
        if let Self::Initialized { profile, .. } = self {
            profile.clone()
        } else {
            None
        }
    }

    pub fn set_profile_content(&mut self, section: ProfileSection, content: ProfileContent) {
        if let Self::Initialized {
            profile: Some(profile),
            ..
        } = self
        {
            profile.set_content(section, content);
        }
    }

    pub fn move_profile_up(&mut self) {
        if let Self::Initialized {
            profile: Some(profile),
            ..
        } = self
        {
            profile.position = profile.position.saturating_sub(1);
            profile
                .list_state
                .select((!profile.content.is_empty()).then_some(profile.position));
        }
    }

    pub fn move_profile_down(&mut self) {
        if let Self::Initialized {
            profile: Some(profile),
            ..
        } = self
        {
            if profile.position + 1 < profile.content.len() {
                profile.position += 1;
                profile.list_state.select(Some(profile.position));
            }
        }
    }

    pub fn move_profile_by(&mut self, delta: isize) {
        if let Self::Initialized {
            profile: Some(profile),
            ..
        } = self
        {
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
