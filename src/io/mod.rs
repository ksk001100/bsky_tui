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
    Search(String, SearchEvent),
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
    Load,
    Next,
    Prev,
    Reload,
}
