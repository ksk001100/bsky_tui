pub mod handler;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IoEvent {
    Initialize,
    LoadTimeline(TimelineEvent),
    LoadNotifications(NotificationEvent),
    LoadNotificationSettings(atrium_api::types::string::Did, String),
    SaveNotificationPreferences(Box<atrium_api::app::bsky::notification::defs::Preferences>),
    SaveActivitySubscription {
        subject: atrium_api::types::string::Did,
        activity: atrium_api::app::bsky::notification::defs::ActivitySubscription,
    },
    ToggleNotificationFollow(atrium_api::types::string::Did),
    LikeNotificationAuthor(atrium_api::types::string::Did),
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
    Moderate(ModerationAction),
    LoadFeedCatalog,
    SearchFeeds(String),
    SelectFeed(crate::app::feed::FeedDescriptor),
    ToggleSavedFeed(crate::app::feed::FeedDescriptor),
    DeletePost(String),
    PreviewLink(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ModerationAction {
    MuteActor {
        did: atrium_api::types::string::Did,
        muted: bool,
    },
    BlockActor {
        did: atrium_api::types::string::Did,
        blocking_uri: Option<String>,
    },
    ReportActor(atrium_api::types::string::Did),
    ReportPost {
        uri: String,
        cid: atrium_api::types::string::Cid,
    },
}

impl ModerationAction {
    pub fn confirmation(&self) -> String {
        match self {
            Self::MuteActor { did, muted: false } => format!("Mute {}?", did.as_str()),
            Self::MuteActor { did, muted: true } => format!("Unmute {}?", did.as_str()),
            Self::BlockActor {
                did,
                blocking_uri: None,
            } => format!("Block {}? This also prevents interaction.", did.as_str()),
            Self::BlockActor {
                did,
                blocking_uri: Some(_),
            } => format!("Unblock {}?", did.as_str()),
            Self::ReportActor(did) => {
                format!("Report profile {} as other violation?", did.as_str())
            }
            Self::ReportPost { uri, .. } => format!("Report post {uri} as other violation?"),
        }
    }
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
