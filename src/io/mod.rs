pub mod handler;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IoEvent {
    Initialize,
    LoadTimeline(TimelineEvent),
    LoadNotifications(NotificationEvent),
    SendPost,
    Like,
    Repost,
    Reply,
    Search(SearchEvent),
    SearchLike,
    SearchRepost,
    SearchReply,
    LoadThread(String),
    LoadInteractions(InteractionKind, String, atrium_api::types::string::Cid),
    SearchUsers(String),
    LoadProfile(atrium_api::types::string::AtIdentifier),
    LoadProfileSection(crate::app::profile::ProfileSection),
    ToggleFollow,
    LoadConnections(InteractionKind, atrium_api::types::string::AtIdentifier),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InteractionKind {
    Likes,
    Reposts,
    Quotes,
    Users,
    Followers,
    Follows,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NotificationEvent {
    Load,
    Next,
    Prev,
    Reload,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimelineEvent {
    Load,
    Next,
    Prev,
    Reload,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SearchEvent {
    Load(String),
    Next,
    Prev,
    Reload,
}
