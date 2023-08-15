pub mod handler;

#[derive(Debug, Clone)]
pub enum IoEvent {
    Initialize,
    LoadTimeline,
    LoadNotifications,
    SendPost,
    Like,
    Repost,
    Reply,
}
