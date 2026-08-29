use atrium_api::app::bsky::{actor::defs::ProfileViewDetailed, feed::defs::FeedViewPost};
use ratatui::widgets::ListState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileSection {
    Posts,
    Replies,
    Media,
    Likes,
    Feeds,
    Lists,
    StarterPacks,
}

impl ProfileSection {
    pub const ALL: [Self; 7] = [
        Self::Posts,
        Self::Replies,
        Self::Media,
        Self::Likes,
        Self::Feeds,
        Self::Lists,
        Self::StarterPacks,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Posts => "Posts",
            Self::Replies => "Replies",
            Self::Media => "Media",
            Self::Likes => "Likes",
            Self::Feeds => "Feeds",
            Self::Lists => "Lists",
            Self::StarterPacks => "Starter Packs",
        }
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0);
        Self::ALL[index.saturating_sub(1)]
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0);
        Self::ALL[(index + 1).min(Self::ALL.len() - 1)]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileListItem {
    pub title: String,
    pub subtitle: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileContent {
    Posts(Vec<FeedViewPost>),
    Items(Vec<ProfileListItem>),
}

impl ProfileContent {
    pub fn len(&self) -> usize {
        match self {
            Self::Posts(posts) => posts.len(),
            Self::Items(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone)]
pub struct ProfileState {
    pub details: ProfileViewDetailed,
    pub section: ProfileSection,
    pub content: ProfileContent,
    pub list_state: ListState,
    pub position: usize,
}

impl ProfileState {
    pub fn new(details: ProfileViewDetailed, content: ProfileContent) -> Self {
        let selected = (!content.is_empty()).then_some(0);
        Self {
            details,
            section: ProfileSection::Posts,
            content,
            list_state: ListState::default().with_selected(selected),
            position: 0,
        }
    }

    pub fn set_content(&mut self, section: ProfileSection, content: ProfileContent) {
        self.section = section;
        self.position = 0;
        self.list_state.select((!content.is_empty()).then_some(0));
        self.content = content;
    }
}

#[cfg(test)]
mod tests {
    use super::ProfileSection;

    #[test]
    fn profile_section_navigation_stays_within_bounds() {
        assert_eq!(ProfileSection::Posts.previous(), ProfileSection::Posts);
        assert_eq!(ProfileSection::Posts.next(), ProfileSection::Replies);
        assert_eq!(
            ProfileSection::StarterPacks.next(),
            ProfileSection::StarterPacks
        );
    }
}
