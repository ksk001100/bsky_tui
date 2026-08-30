use std::sync::Arc;

use atrium_api::{app::bsky::feed::post::ReplyRefData, com::atproto::repo::strong_ref};
use bsky_sdk::BskyAgent;
use eyre::{eyre, Result, WrapErr};

use super::{
    FeatureEvent, InteractionKind, IoEvent, NotificationEvent, SearchEvent, TimelineEvent,
};
use crate::{
    app::{
        auth::AuthCredentials, command::EffectContext, config::AppConfig, message::EffectMessage,
        profile::ProfileSection, profile::ProfileState, state::Mode, state::Tab,
    },
    bsky::{self, TimelineClient},
};

pub struct IoAsyncHandler {
    effect_tx: tokio::sync::mpsc::Sender<EffectEnvelope>,
    context: Option<EffectContext>,
}

pub struct EffectEnvelope {
    pub message: EffectMessage,
    pub applied: tokio::sync::oneshot::Sender<EffectContext>,
}

impl IoAsyncHandler {
    pub fn new(effect_tx: tokio::sync::mpsc::Sender<EffectEnvelope>) -> Self {
        Self {
            effect_tx,
            context: None,
        }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent, context: EffectContext) {
        self.context = Some(context);
        let operation = operation_name(&io_event);
        let result = match io_event {
            IoEvent::Initialize => self.initialize().await,
            IoEvent::LoadTimeline(action) => self.load_timeline(action).await,
            IoEvent::SendPost => self.send_post().await,
            IoEvent::LoadNotifications(action) => self.load_notifications(action).await,
            IoEvent::LoadNotificationSettings(subject, handle) => {
                self.load_notification_settings(subject, handle).await
            }
            IoEvent::SaveNotificationPreferences(preferences) => {
                self.save_notification_preferences(*preferences).await
            }
            IoEvent::SaveActivitySubscription { subject, activity } => {
                self.save_activity_subscription(subject, activity).await
            }
            IoEvent::ToggleNotificationFollow(subject) => {
                self.toggle_notification_follow(subject).await
            }
            IoEvent::LikeNotificationAuthor(subject) => {
                self.like_notification_author(subject).await
            }
            IoEvent::Like => self.like().await,
            IoEvent::Repost => self.repost().await,
            IoEvent::Reply => self.reply().await,
            IoEvent::Search(action) => self.search(action).await,
            IoEvent::SearchLike => self.search_like().await,
            IoEvent::SearchRepost => self.search_repost().await,
            IoEvent::SearchReply => self.search_reply().await,
            IoEvent::LoadThread(uri) => self.load_thread(uri).await,
            IoEvent::LoadInteractions(kind, uri, cid) => {
                self.load_interactions(kind, uri, cid).await
            }
            IoEvent::SearchUsers(query) => self.search_users(query).await,
            IoEvent::LoadProfile(actor) => self.load_profile(actor).await,
            IoEvent::LoadProfileSection(section) => self.load_profile_section(section).await,
            IoEvent::ToggleFollow => self.toggle_follow().await,
            IoEvent::LoadConnections(kind, actor) => self.load_connections(kind, actor).await,
            IoEvent::Moderate(action) => self.moderate(action).await,
            IoEvent::LoadFeedCatalog => self.load_feed_catalog(true).await,
            IoEvent::SearchFeeds(query) => self.search_feeds(query).await,
            IoEvent::SelectFeed(feed) => self.select_feed(feed).await,
            IoEvent::ToggleSavedFeed(feed) => self.toggle_saved_feed(feed).await,
            IoEvent::DeletePost(uri) => self.delete_post(uri).await,
            IoEvent::PreviewLink(url) => self.preview_link(url).await,
            IoEvent::Feature(event) => self.handle_feature_event(event).await,
        };

        let error = match result {
            Ok(()) => {
                crate::logging::event(operation, true);
                None
            }
            Err(error) => {
                crate::logging::event(operation, false);
                Some(user_error(operation, &error))
            }
        };
        self.emit(EffectMessage::Finished { error }).await;
    }

    async fn emit(&mut self, message: EffectMessage) {
        let (applied, acknowledgement) = tokio::sync::oneshot::channel();
        if self
            .effect_tx
            .send(EffectEnvelope { message, applied })
            .await
            .is_ok()
        {
            if let Ok(context) = acknowledgement.await {
                self.context = Some(context);
            }
        }
    }

    fn state(&self) -> &crate::app::state::AppState {
        &self
            .context
            .as_ref()
            .expect("effect context is set before handling a command")
            .state
    }

    async fn agent(&self) -> Result<Arc<BskyAgent>> {
        self.state()
            .get_agent()
            .ok_or_else(|| eyre!("client is not initialized"))
    }

    async fn did(&self) -> Result<atrium_api::types::string::Did> {
        self.state()
            .get_did()
            .ok_or_else(|| eyre!("account is not initialized"))
    }
}

mod composer;
mod features;
mod notifications;
mod social;
mod timeline;

fn setting_row(
    title: &str,
    value: String,
    key: crate::app::feature_panel::SettingKey,
) -> crate::app::feature_panel::FeatureRow {
    crate::app::feature_panel::FeatureRow {
        title: title.to_owned(),
        detail: value,
        target: crate::app::feature_panel::FeatureTarget::Setting(key),
        unread: false,
    }
}

fn drafts_with_default_language(text: &str) -> Result<Vec<crate::app::composer::PostDraft>> {
    let mut drafts = crate::app::composer::parse_drafts(text)?;
    let language = crate::app::config::AppConfig::load()?.ui.language;
    if language != "auto" && !language.trim().is_empty() {
        let language =
            atrium_api::types::string::Language::new(language).map_err(eyre::Report::msg)?;
        for draft in &mut drafts {
            if draft.langs.is_none() {
                draft.langs = Some(vec![language.clone()]);
            }
        }
    }
    Ok(drafts)
}

fn updated_cursors(
    mut cursors: Vec<Option<String>>,
    target: usize,
    current: Option<String>,
    next: Option<String>,
) -> Vec<Option<String>> {
    cursors.resize(target + 1, None);
    cursors[target] = current;
    cursors.truncate(target + 1);
    cursors.push(next);
    cursors
}

fn count_new_posts(
    old: &[atrium_api::app::bsky::feed::defs::FeedViewPost],
    new: &[atrium_api::app::bsky::feed::defs::FeedViewPost],
) -> usize {
    let Some(first_old_uri) = old.first().map(|feed| feed.post.uri.as_str()) else {
        return 0;
    };
    new.iter()
        .take_while(|feed| feed.post.uri != first_old_uri)
        .count()
}

fn operation_name(event: &IoEvent) -> &'static str {
    match event {
        IoEvent::Initialize => "Initialization",
        IoEvent::LoadTimeline(_) => "Timeline request",
        IoEvent::LoadNotifications(_) => "Notification request",
        IoEvent::LoadNotificationSettings(_, _) => "Notification settings request",
        IoEvent::SaveNotificationPreferences(_) => "Notification settings update",
        IoEvent::SaveActivitySubscription { .. } => "Activity notification update",
        IoEvent::ToggleNotificationFollow(_) => "Follow update",
        IoEvent::LikeNotificationAuthor(_) => "Like back",
        IoEvent::SendPost => "Post",
        IoEvent::Like | IoEvent::SearchLike => "Like",
        IoEvent::Repost | IoEvent::SearchRepost => "Repost",
        IoEvent::Reply | IoEvent::SearchReply => "Reply",
        IoEvent::LoadThread(_) => "Thread request",
        IoEvent::LoadInteractions(_, _, _) => "Post interactions request",
        IoEvent::SearchUsers(_) => "User search",
        IoEvent::LoadProfile(_) | IoEvent::LoadProfileSection(_) => "Profile request",
        IoEvent::ToggleFollow => "Follow request",
        IoEvent::LoadConnections(_, _) => "Social graph request",
        IoEvent::Moderate(_) => "Moderation request",
        IoEvent::LoadFeedCatalog
        | IoEvent::SearchFeeds(_)
        | IoEvent::SelectFeed(_)
        | IoEvent::ToggleSavedFeed(_) => "Feed request",
        IoEvent::DeletePost(_) => "Delete post",
        IoEvent::PreviewLink(_) => "Link preview",
        IoEvent::Search(_) => "Search",
        IoEvent::Feature(_) => "Extended client request",
    }
}

fn user_error(operation: &str, error: &eyre::Report) -> String {
    let detail = format!("{error:#}");
    let lower = detail.to_ascii_lowercase();
    let category = if lower.contains("429") || lower.contains("rate limit") {
        "Rate limit exceeded"
    } else if lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("authentication")
        || lower.contains("invalid identifier or password")
    {
        "Authentication failed"
    } else if lower.contains("timeout")
        || lower.contains("connect")
        || lower.contains("dns")
        || lower.contains("network")
    {
        "Network request failed"
    } else {
        "Operation failed"
    };
    format!("{operation}: {category}. {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_history_is_replaced_after_reload() {
        let cursors = vec![None, Some("page-2".into()), Some("stale".into())];
        let updated = updated_cursors(cursors, 1, Some("page-2".into()), Some("fresh".into()));
        assert_eq!(
            updated,
            vec![None, Some("page-2".into()), Some("fresh".into())]
        );
    }

    #[test]
    fn network_and_authentication_failures_are_classified_without_panicking() {
        let network = user_error("Timeline", &eyre!("connection timeout"));
        assert!(network.contains("Network request failed"));
        let auth = user_error("Initialization", &eyre!("401 unauthorized"));
        assert!(auth.contains("Authentication failed"));
    }
}
