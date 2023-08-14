pub mod handler;

#[derive(Debug, Clone)]
pub enum IoEvent {
    Initialize,
    LoadFeed,
    LoadNotifications,
    SendPost,
    Like,
    Repost,
    Reply,
}
