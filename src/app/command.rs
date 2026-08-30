//! Side effects emitted by the application update layer.

use std::sync::Arc;

use atrium_api::{
    app::bsky::feed::defs::{FeedViewPost, PostViewData},
    types::string::Did,
};
use bsky_sdk::BskyAgent;

use super::{
    feature_panel::FeatureSection,
    feed::FeedDescriptor,
    moderation::ModerationPrefs,
    profile::{ProfileSection, ProfileState},
    state::{AppState, Mode, Tab},
};
use crate::io::{FeatureEvent, IoEvent};

#[derive(Clone)]
pub struct TimelineEffectContext {
    pub current_cursor_index: usize,
    pub cursors: Vec<Option<String>>,
    pub active_feed: FeedDescriptor,
    pub posts: Vec<FeedViewPost>,
    pub position: usize,
    pub loaded: bool,
}

#[derive(Clone)]
pub struct PaginationEffectContext {
    pub current_cursor_index: usize,
    pub cursors: Vec<Option<String>>,
}

#[derive(Clone)]
pub struct SearchEffectContext {
    pub pagination: PaginationEffectContext,
    pub query: Option<String>,
}

/// Immutable state required by one I/O command and its follow-up effects.
///
/// Large collections are absent unless the command can actually use them.
#[derive(Clone)]
pub struct EffectContext {
    pub agent: Option<Arc<BskyAgent>>,
    pub did: Option<Did>,
    pub moderation: Option<ModerationPrefs>,
    pub input: Option<String>,
    pub timeline: Option<TimelineEffectContext>,
    pub notifications: Option<PaginationEffectContext>,
    pub search: Option<SearchEffectContext>,
    pub current_feed: Option<FeedViewPost>,
    pub current_search_result: Option<PostViewData>,
    pub mode: Mode,
    pub tab: Tab,
    pub profile: Option<ProfileState>,
    pub profile_did: Option<Did>,
    pub profile_section: Option<ProfileSection>,
    pub feature_panel_open: bool,
    pub feature_panel_section: Option<FeatureSection>,
}

impl EffectContext {
    pub fn for_event(
        state: &AppState,
        event: &IoEvent,
        feature_panel_open: bool,
        feature_panel_section: Option<FeatureSection>,
    ) -> Self {
        let needs_timeline = matches!(
            event,
            IoEvent::Initialize
                | IoEvent::LoadTimeline(_)
                | IoEvent::SendPost
                | IoEvent::Like
                | IoEvent::Repost
                | IoEvent::Reply
                | IoEvent::SelectFeed(_)
                | IoEvent::DeletePost(_)
                | IoEvent::Moderate(_)
                | IoEvent::Feature(
                    FeatureEvent::UseListFeed { .. } | FeatureEvent::SwitchAccount(_)
                )
        );
        let needs_notifications = matches!(
            event,
            IoEvent::LoadNotifications(_) | IoEvent::ToggleNotificationFollow(_)
        );
        let needs_search = matches!(
            event,
            IoEvent::Search(_)
                | IoEvent::SearchLike
                | IoEvent::SearchRepost
                | IoEvent::SearchReply
                | IoEvent::DeletePost(_)
                | IoEvent::Moderate(_)
        );
        let needs_profile = matches!(event, IoEvent::ToggleFollow | IoEvent::Moderate(_));
        let needs_profile_identity = needs_profile
            || matches!(
                event,
                IoEvent::LoadProfileSection(_) | IoEvent::DeletePost(_)
            );
        let needs_timeline_posts = matches!(
            event,
            IoEvent::LoadTimeline(crate::io::TimelineEvent::Reload)
                | IoEvent::SendPost
                | IoEvent::Like
                | IoEvent::Repost
                | IoEvent::Reply
                | IoEvent::DeletePost(_)
                | IoEvent::Moderate(_)
        );
        let needs_moderation = needs_timeline
            || needs_notifications
            || needs_search
            || matches!(
                event,
                IoEvent::LoadThread(_)
                    | IoEvent::LoadProfile(_)
                    | IoEvent::LoadProfileSection(_)
                    | IoEvent::Feature(
                        FeatureEvent::ToggleHiddenReply { .. } | FeatureEvent::DetachQuote { .. }
                    )
            );

        let timeline = needs_timeline.then(|| {
            let (loaded, posts) = match state {
                AppState::Initialized { timeline, .. } => (
                    timeline.is_some(),
                    if needs_timeline_posts {
                        timeline.clone().unwrap_or_default()
                    } else {
                        Vec::new()
                    },
                ),
                AppState::Init => (false, Vec::new()),
            };
            TimelineEffectContext {
                current_cursor_index: state.get_tl_current_cursor_index(),
                cursors: state.get_cursors(),
                active_feed: state.get_active_feed(),
                loaded,
                posts,
                position: state.get_tl_list_position(),
            }
        });
        let (profile_did, profile_section) = if needs_profile_identity {
            match state {
                AppState::Initialized {
                    profile: Some(profile),
                    ..
                } => (Some(profile.details.did.clone()), Some(profile.section)),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        Self {
            agent: state.get_agent(),
            did: state.get_did(),
            moderation: needs_moderation.then(|| state.moderation()),
            input: matches!(
                event,
                IoEvent::SendPost | IoEvent::Reply | IoEvent::SearchReply
            )
            .then(|| state.get_input().value().to_owned()),
            timeline,
            notifications: needs_notifications.then(|| PaginationEffectContext {
                current_cursor_index: state.get_notifications_current_cursor_index(),
                cursors: state.get_notification_cursors(),
            }),
            search: needs_search.then(|| SearchEffectContext {
                pagination: PaginationEffectContext {
                    current_cursor_index: state.get_search_current_cursor_index(),
                    cursors: state.get_search_cursors(),
                },
                query: state.get_search_query(),
            }),
            current_feed: matches!(event, IoEvent::Like | IoEvent::Repost | IoEvent::Reply)
                .then(|| state.get_current_feed())
                .flatten(),
            current_search_result: matches!(
                event,
                IoEvent::SearchLike | IoEvent::SearchRepost | IoEvent::SearchReply
            )
            .then(|| state.get_current_search_result())
            .flatten(),
            mode: state.get_mode(),
            tab: state.get_tab(),
            profile: needs_profile.then(|| state.get_profile()).flatten(),
            profile_did,
            profile_section,
            feature_panel_open,
            feature_panel_section,
        }
    }

    pub fn get_agent(&self) -> Option<Arc<BskyAgent>> {
        self.agent.clone()
    }

    pub fn get_did(&self) -> Option<Did> {
        self.did.clone()
    }

    pub fn moderation(&self) -> ModerationPrefs {
        self.moderation.clone().unwrap_or_default()
    }

    pub fn get_tl_current_cursor_index(&self) -> usize {
        self.timeline
            .as_ref()
            .map_or(0, |timeline| timeline.current_cursor_index)
    }

    pub fn get_cursors(&self) -> Vec<Option<String>> {
        self.timeline
            .as_ref()
            .map_or_else(Vec::new, |timeline| timeline.cursors.clone())
    }

    pub fn get_active_feed(&self) -> FeedDescriptor {
        self.timeline
            .as_ref()
            .map_or_else(FeedDescriptor::following, |timeline| {
                timeline.active_feed.clone()
            })
    }

    pub fn get_timeline(&self) -> Option<Vec<FeedViewPost>> {
        self.timeline
            .as_ref()
            .and_then(|timeline| timeline.loaded.then(|| timeline.posts.clone()))
    }

    pub fn get_tl_list_position(&self) -> usize {
        self.timeline
            .as_ref()
            .map_or(0, |timeline| timeline.position)
    }

    pub fn get_notifications_current_cursor_index(&self) -> usize {
        self.notifications
            .as_ref()
            .map_or(0, |pagination| pagination.current_cursor_index)
    }

    pub fn get_notification_cursors(&self) -> Vec<Option<String>> {
        self.notifications
            .as_ref()
            .map_or_else(Vec::new, |pagination| pagination.cursors.clone())
    }

    pub fn get_search_current_cursor_index(&self) -> usize {
        self.search
            .as_ref()
            .map_or(0, |search| search.pagination.current_cursor_index)
    }

    pub fn get_search_cursors(&self) -> Vec<Option<String>> {
        self.search
            .as_ref()
            .map_or_else(Vec::new, |search| search.pagination.cursors.clone())
    }

    pub fn get_search_query(&self) -> Option<String> {
        self.search.as_ref().and_then(|search| search.query.clone())
    }

    pub fn get_current_feed(&self) -> Option<FeedViewPost> {
        self.current_feed.clone()
    }

    pub fn get_current_search_result(&self) -> Option<PostViewData> {
        self.current_search_result.clone()
    }

    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    pub fn get_tab(&self) -> Tab {
        self.tab
    }

    pub fn get_profile(&self) -> Option<ProfileState> {
        self.profile.clone()
    }
}

/// Work delegated to the asynchronous I/O runtime.
///
/// Keeping commands as data makes the state-transition boundary explicit and
/// allows future reducers to be tested without performing network I/O.
pub enum Command {
    Io {
        event: IoEvent,
        context: Box<EffectContext>,
    },
    LoadImages(Vec<String>),
    PollImages,
    OpenUrl {
        url: String,
        error_context: &'static str,
    },
    CopyToClipboard {
        value: String,
        label: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn commands_are_forwarded_to_the_effect_runtime() {
        let mut app = App::new();

        let update = app.init();

        let [Command::Io {
            event: IoEvent::Initialize,
            context,
        }] = update.commands.as_slice()
        else {
            panic!("initialization should emit exactly one I/O command");
        };
        assert!(context.timeline.is_some());
        assert!(context.notifications.is_none());
        assert!(context.search.is_none());
        assert!(context.profile.is_none());
        assert!(app.is_loading());
    }

    #[test]
    fn effect_context_only_includes_state_for_its_event() {
        let app = App::new();

        let notification_context = app.effect_context(&IoEvent::LoadNotifications(
            crate::io::NotificationEvent::Load,
        ));
        assert!(notification_context.notifications.is_some());
        assert!(notification_context.timeline.is_none());
        assert!(notification_context.search.is_none());
        assert!(notification_context.profile.is_none());

        let search_context = app.effect_context(&IoEvent::Search(crate::io::SearchEvent::Reload));
        assert!(search_context.search.is_some());
        assert!(search_context.timeline.is_none());
        assert!(search_context.notifications.is_none());
        assert!(search_context.profile.is_none());
    }
}
