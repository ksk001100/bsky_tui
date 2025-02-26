pub mod handler;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IoEvent {
    Initialize,
    LoadTimeline(TimelineEvent),
    LoadNotifications,
    SendPost,
    Like,
    Repost,
    Reply,
    Search(SearchEvent),
    SearchLike,
    SearchRepost,
    SearchReply,
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
