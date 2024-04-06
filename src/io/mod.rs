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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimelineEvent {
    Load,
    Next,
    Prev,
    Reload,
}
