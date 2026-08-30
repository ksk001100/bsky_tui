use std::collections::HashMap;

use atrium_api::app::bsky::notification::{
    defs::{ActivitySubscription, Preferences},
    list_notifications::Notification,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReasonFilter {
    #[default]
    All,
    Reply,
    Mention,
    Quote,
    Like,
    Repost,
    Follow,
    LikeViaRepost,
    RepostViaRepost,
    StarterpackJoined,
    Verified,
    Unverified,
    Activity,
}

impl ReasonFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Reply,
            Self::Reply => Self::Mention,
            Self::Mention => Self::Quote,
            Self::Quote => Self::Like,
            Self::Like => Self::Repost,
            Self::Repost => Self::Follow,
            Self::Follow => Self::LikeViaRepost,
            Self::LikeViaRepost => Self::RepostViaRepost,
            Self::RepostViaRepost => Self::StarterpackJoined,
            Self::StarterpackJoined => Self::Verified,
            Self::Verified => Self::Unverified,
            Self::Unverified => Self::Activity,
            Self::Activity => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Reply => "reply",
            Self::Mention => "mention",
            Self::Quote => "quote",
            Self::Like => "like",
            Self::Repost => "repost",
            Self::Follow => "follow",
            Self::LikeViaRepost => "like-via-repost",
            Self::RepostViaRepost => "repost-via-repost",
            Self::StarterpackJoined => "starterpack-joined",
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::Activity => "activity",
        }
    }

    fn matches(self, reason: &str) -> bool {
        self == Self::All
            || self.label() == reason
            || (self == Self::Activity && reason == "subscribed-post")
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SenderFilter {
    #[default]
    All,
    Following,
    Others,
}

impl SenderFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Following,
            Self::Following => Self::Others,
            Self::Others => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Following => "following",
            Self::Others => "others",
        }
    }

    fn matches(self, notification: &Notification) -> bool {
        let following = notification
            .author
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.following.as_ref())
            .is_some();
        match self {
            Self::All => true,
            Self::Following => following,
            Self::Others => !following,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReadFilter {
    #[default]
    All,
    Unread,
    Read,
}

impl ReadFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Unread,
            Self::Unread => Self::Read,
            Self::Read => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Unread => "unread",
            Self::Read => "read",
        }
    }

    fn matches(self, notification: &Notification) -> bool {
        match self {
            Self::All => true,
            Self::Unread => !notification.is_read,
            Self::Read => notification.is_read,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NotificationFilters {
    pub reason: ReasonFilter,
    pub sender: SenderFilter,
    pub read: ReadFilter,
}

#[derive(Clone, Debug)]
pub struct NotificationGroup {
    pub notifications: Vec<Notification>,
}

impl NotificationGroup {
    pub fn primary(&self) -> &Notification {
        &self.notifications[0]
    }
}

pub fn groups(
    notifications: &[Notification],
    filters: NotificationFilters,
) -> Vec<NotificationGroup> {
    let mut result: Vec<NotificationGroup> = Vec::new();
    let mut grouped_indices: HashMap<String, usize> = HashMap::new();
    for notification in notifications.iter().filter(|notification| {
        filters.reason.matches(&notification.reason)
            && filters.sender.matches(notification)
            && filters.read.matches(notification)
    }) {
        let key = group_key(&notification.reason, notification.reason_subject.as_deref());
        if let Some(index) = key
            .as_ref()
            .and_then(|key| grouped_indices.get(key))
            .copied()
        {
            result[index].notifications.push(notification.clone());
        } else {
            let index = result.len();
            result.push(NotificationGroup {
                notifications: vec![notification.clone()],
            });
            if let Some(key) = key {
                grouped_indices.insert(key, index);
            }
        }
    }
    result
}

pub fn post_uri(notification: &Notification) -> Option<&str> {
    match notification.reason.as_str() {
        "reply" | "mention" | "quote" | "subscribed-post" => Some(&notification.uri),
        "like" | "repost" | "like-via-repost" | "repost-via-repost" => {
            notification.reason_subject.as_deref()
        }
        _ => None,
    }
}

fn group_key(reason: &str, subject: Option<&str>) -> Option<String> {
    matches!(reason, "like" | "repost" | "follow")
        .then(|| format!("{reason}:{}", subject.unwrap_or("account")))
}

#[derive(Clone, Debug)]
pub struct NotificationSettings {
    pub preferences: Preferences,
    pub category: usize,
    pub activity_subject: Option<(atrium_api::types::string::Did, String, ActivitySubscription)>,
}

pub const PREFERENCE_CATEGORIES: [&str; 13] = [
    "like",
    "repost",
    "follow",
    "mention",
    "reply",
    "quote",
    "like-via-repost",
    "repost-via-repost",
    "subscribed-post",
    "starterpack-joined",
    "verified",
    "unverified",
    "chat",
];

impl NotificationSettings {
    pub fn previous(&mut self) {
        self.category = self.category.saturating_sub(1);
    }

    pub fn next(&mut self) {
        if self.category + 1 < PREFERENCE_CATEGORIES.len() {
            self.category += 1;
        }
    }

    pub fn selected(&self) -> &'static str {
        PREFERENCE_CATEGORIES[self.category]
    }

    pub fn toggle_list(&mut self) {
        match self.category {
            0 => self.preferences.like.list ^= true,
            1 => self.preferences.repost.list ^= true,
            2 => self.preferences.follow.list ^= true,
            3 => self.preferences.mention.list ^= true,
            4 => self.preferences.reply.list ^= true,
            5 => self.preferences.quote.list ^= true,
            6 => self.preferences.like_via_repost.list ^= true,
            7 => self.preferences.repost_via_repost.list ^= true,
            8 => self.preferences.subscribed_post.list ^= true,
            9 => self.preferences.starterpack_joined.list ^= true,
            10 => self.preferences.verified.list ^= true,
            11 => self.preferences.unverified.list ^= true,
            _ => {}
        }
    }

    pub fn toggle_push(&mut self) {
        match self.category {
            0 => self.preferences.like.push ^= true,
            1 => self.preferences.repost.push ^= true,
            2 => self.preferences.follow.push ^= true,
            3 => self.preferences.mention.push ^= true,
            4 => self.preferences.reply.push ^= true,
            5 => self.preferences.quote.push ^= true,
            6 => self.preferences.like_via_repost.push ^= true,
            7 => self.preferences.repost_via_repost.push ^= true,
            8 => self.preferences.subscribed_post.push ^= true,
            9 => self.preferences.starterpack_joined.push ^= true,
            10 => self.preferences.verified.push ^= true,
            11 => self.preferences.unverified.push ^= true,
            12 => self.preferences.chat.push ^= true,
            _ => {}
        }
    }

    pub fn cycle_include(&mut self) {
        fn next(value: &mut String) {
            *value = match value.as_str() {
                "all" => "follows",
                _ => "all",
            }
            .to_owned();
        }
        match self.category {
            0 => next(&mut self.preferences.like.include),
            1 => next(&mut self.preferences.repost.include),
            2 => next(&mut self.preferences.follow.include),
            3 => next(&mut self.preferences.mention.include),
            4 => next(&mut self.preferences.reply.include),
            5 => next(&mut self.preferences.quote.include),
            6 => next(&mut self.preferences.like_via_repost.include),
            7 => next(&mut self.preferences.repost_via_repost.include),
            12 => next(&mut self.preferences.chat.include),
            _ => {}
        }
    }

    pub fn cycle_activity(&mut self) {
        if let Some((_, _, activity)) = &mut self.activity_subject {
            (activity.post, activity.reply) = match (activity.post, activity.reply) {
                (false, false) => (true, false),
                (true, false) => (false, true),
                (false, true) => (true, true),
                (true, true) => (false, false),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_cycles_are_closed() {
        let mut reason = ReasonFilter::All;
        for _ in 0..13 {
            reason = reason.next();
        }
        assert_eq!(reason, ReasonFilter::All);

        let mut sender = SenderFilter::All;
        let mut read = ReadFilter::All;
        for _ in 0..3 {
            sender = sender.next();
            read = read.next();
        }
        assert_eq!(sender, SenderFilter::All);
        assert_eq!(read, ReadFilter::All);
    }

    #[test]
    fn only_groupable_reasons_share_a_subject_bucket() {
        assert_eq!(
            group_key("like", Some("at://did/post/1")),
            Some("like:at://did/post/1".to_owned())
        );
        assert_eq!(group_key("follow", None), Some("follow:account".to_owned()));
        assert_eq!(group_key("reply", Some("at://did/post/1")), None);
    }
}
