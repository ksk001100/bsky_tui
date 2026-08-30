//! Events accepted by the application update loop.

use crossterm::event::MouseEvent;

use std::collections::HashMap;

use atrium_api::{
    app::bsky::{
        feed::defs::{FeedViewPost, PostViewData},
        notification::list_notifications::Notification,
    },
    types::string::{Did, Handle},
};
use bsky_sdk::BskyAgent;

use super::{
    config::UiConfig,
    feature_panel::FeatureRow,
    feed::FeedDescriptor,
    moderation::ModerationPrefs,
    notifications::NotificationSettings,
    profile::{ProfileContent, ProfileSection, ProfileState},
    thread::ThreadEntry,
};
use crate::{bsky, inputs::key::Key, io::InteractionKind};

/// An event that may change the application model.
pub enum Message {
    KeyPressed(Key),
    Mouse(MouseEvent),
    Tick,
    Effect(Box<EffectMessage>),
}

impl Message {
    pub fn effect(message: EffectMessage) -> Self {
        Self::Effect(Box::new(message))
    }
}

/// Typed results produced by the asynchronous effect runtime.
pub enum EffectMessage {
    Finished {
        error: Option<String>,
    },
    RuntimeError(String),
    Initialized {
        agent: BskyAgent,
        handle: Handle,
        did: Did,
        moderation: ModerationPrefs,
        ui_config: UiConfig,
    },
    TimelineLoaded {
        posts: Vec<FeedViewPost>,
        position: usize,
        new_count: usize,
        cursors: Vec<Option<String>>,
        cursor_index: usize,
        image_urls: Vec<String>,
    },
    FeedCatalogLoaded {
        catalog: Vec<FeedDescriptor>,
        open: bool,
    },
    FeedSearchLoaded(Vec<FeedDescriptor>),
    FeedActivated(FeedDescriptor),
    ThreadClosed,
    ComposerPreviewLoaded(bsky::LinkPreview),
    ThreadLoaded {
        entries: Vec<ThreadEntry>,
        image_urls: Vec<String>,
    },
    InteractionsLoaded {
        kind: InteractionKind,
        items: Vec<bsky::InteractionItem>,
    },
    UserSearchLoaded(Vec<bsky::InteractionItem>),
    ProfileLoaded {
        profile: ProfileState,
        image_urls: Vec<String>,
    },
    ProfileContentLoaded {
        section: ProfileSection,
        content: ProfileContent,
        image_urls: Vec<String>,
    },
    ProfileUpdated(ProfileState),
    ComposerFinished,
    NotificationsLoaded {
        notifications: Vec<Notification>,
        posts: HashMap<String, PostViewData>,
        cursors: Vec<Option<String>>,
        cursor_index: usize,
        image_urls: Vec<String>,
    },
    NotificationSettingsLoaded(NotificationSettings),
    SearchLoaded {
        query: String,
        posts: Vec<PostViewData>,
        cursors: Vec<Option<String>>,
        cursor_index: usize,
        image_urls: Vec<String>,
    },
    FeaturePanelClosed,
    FeatureRowsLoaded {
        title: String,
        rows: Vec<FeatureRow>,
        child: bool,
    },
    ConversationLoaded {
        title: String,
        rows: Vec<FeatureRow>,
    },
    MessagesReplaced {
        title: String,
        rows: Vec<FeatureRow>,
    },
    ExploreReplaced(Vec<FeatureRow>),
    UiConfigLoaded(UiConfig),
}
