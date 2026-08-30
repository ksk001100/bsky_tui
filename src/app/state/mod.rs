use std::{collections::HashMap, fmt, sync::Arc};

use atrium_api::{
    app::bsky::{
        feed::defs::{FeedViewPost, PostViewData},
        notification::list_notifications::Notification,
    },
    types::string::{Did, Handle},
};
use bsky_sdk::BskyAgent;
use ratatui::widgets::ListState;
use tui_input::{Input, InputRequest};

use crate::app::feed::{FeedDescriptor, FeedKind, FeedSnapshot};
use crate::app::moderation::ModerationPrefs;
use crate::app::notifications::{self, NotificationFilters, NotificationGroup};
use crate::app::profile::{ProfileContent, ProfileSection, ProfileState};
use crate::app::thread::ThreadEntry;

const MAX_CURSOR_HISTORY: usize = 20;
const HALF_PAGE_ITEMS: usize = 5;
const MAX_FEED_SNAPSHOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Post,
    Reply,
    Help,
    Search,
    UserSearch,
    Thread,
    Profile,
    FeedSearch,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Mode::Normal => "Normal",
            Mode::Post => "Post",
            Mode::Reply => "Reply",
            Mode::Help => "Help",
            Mode::Search => "Search",
            Mode::UserSearch => "User Search",
            Mode::Thread => "Thread",
            Mode::Profile => "Profile",
            Mode::FeedSearch => "Feed Search",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Notifications,
    Messages,
    Search,
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Tab::Home => "Home",
            Tab::Notifications => "Notifications",
            Tab::Messages => "Messages",
            Tab::Search => "Explore",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, Default)]
pub enum AppState {
    #[default]
    Init,
    Initialized(Box<Model>),
}

#[derive(Clone)]
pub struct Model {
    session: SessionState,
    navigation: NavigationState,
    composer: ComposerState,
    home: HomeState,
    notifications: NotificationState,
    explore: ExploreState,
    thread: ThreadState,
    profile: Option<ProfileState>,
}

#[derive(Clone)]
struct SessionState {
    agent: Arc<BskyAgent>,
    handle: Handle,
    did: Did,
    moderation: ModerationPrefs,
}

#[derive(Clone)]
struct NavigationState {
    mode: Mode,
    tab: Tab,
}

#[derive(Clone, Default)]
struct ComposerState {
    input: Input,
}

#[derive(Clone)]
struct HomeState {
    timeline: Option<Vec<FeedViewPost>>,
    selection: SelectionState,
    pagination: PaginationState,
    active_feed: FeedDescriptor,
    feed_snapshots: HashMap<String, FeedSnapshot>,
}

#[derive(Clone)]
struct NotificationState {
    items: Option<Vec<Notification>>,
    selection: SelectionState,
    pagination: PaginationState,
    filters: NotificationFilters,
    posts: HashMap<String, PostViewData>,
}

#[derive(Clone)]
struct ExploreState {
    results: Option<Vec<PostViewData>>,
    selection: SelectionState,
    pagination: PaginationState,
    query: Option<String>,
}

#[derive(Clone, Default)]
struct ThreadState {
    entries: Option<Vec<ThreadEntry>>,
    selection: SelectionState,
}

#[derive(Clone, Default)]
struct SelectionState {
    list: ListState,
    position: usize,
}

#[derive(Clone)]
struct PaginationState {
    current: usize,
    cursors: Vec<Option<String>>,
}

impl Default for PaginationState {
    fn default() -> Self {
        Self {
            current: 0,
            cursors: vec![None],
        }
    }
}

impl AppState {
    pub fn initialized(
        agent: BskyAgent,
        handle: Handle,
        did: Did,
        moderation: ModerationPrefs,
    ) -> Self {
        let agent = Arc::new(agent);
        let selected = || SelectionState {
            list: ListState::default().with_selected(Some(0)),
            position: 0,
        };
        Self::Initialized(Box::new(Model {
            session: SessionState {
                agent,
                handle,
                did,
                moderation,
            },
            navigation: NavigationState {
                mode: Mode::Normal,
                tab: Tab::Home,
            },
            composer: ComposerState::default(),
            home: HomeState {
                timeline: None,
                selection: selected(),
                pagination: PaginationState::default(),
                active_feed: FeedDescriptor::following(),
                feed_snapshots: HashMap::new(),
            },
            notifications: NotificationState {
                items: None,
                selection: selected(),
                pagination: PaginationState::default(),
                filters: NotificationFilters::default(),
                posts: HashMap::new(),
            },
            explore: ExploreState {
                results: None,
                selection: selected(),
                pagination: PaginationState::default(),
                query: None,
            },
            thread: ThreadState::default(),
            profile: None,
        }))
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized(_))
    }

    pub fn get_handle(&self) -> Option<Handle> {
        if let Self::Initialized(model) = self {
            Some(model.session.handle.clone())
        } else {
            None
        }
    }

    pub fn get_agent(&self) -> Option<Arc<BskyAgent>> {
        if let Self::Initialized(model) = self {
            Some(model.session.agent.clone())
        } else {
            None
        }
    }

    pub fn moderation(&self) -> ModerationPrefs {
        if let Self::Initialized(model) = self {
            model.session.moderation.clone()
        } else {
            ModerationPrefs::default()
        }
    }

    fn model(&self) -> Option<&Model> {
        match self {
            Self::Initialized(model) => Some(model),
            Self::Init => None,
        }
    }

    fn model_mut(&mut self) -> Option<&mut Model> {
        match self {
            Self::Initialized(model) => Some(model),
            Self::Init => None,
        }
    }

    /// Applies a server-confirmed bookmark state to every cached representation
    /// of the post. This is deliberately called only after the XRPC succeeds.
    pub fn set_post_bookmarked(&mut self, uri: &str, bookmarked: bool) {
        let Some(model) = self.model_mut() else {
            return;
        };
        let update = |post: &mut PostViewData| {
            if post.uri != uri {
                return;
            }
            let viewer = post.viewer.get_or_insert_with(|| {
                atrium_api::app::bsky::feed::defs::ViewerStateData {
                    bookmarked: None,
                    embedding_disabled: None,
                    like: None,
                    pinned: None,
                    reply_disabled: None,
                    repost: None,
                    thread_muted: None,
                }
                .into()
            });
            viewer.bookmarked = Some(bookmarked);
        };

        if let Some(posts) = model.home.timeline.as_mut() {
            for feed in posts.iter_mut() {
                update(&mut feed.post.data);
            }
            if !bookmarked && matches!(model.home.active_feed.kind, FeedKind::Bookmarks) {
                posts.retain(|feed| feed.post.uri != uri);
                let length = posts.len();
                model.home.selection.position =
                    model.home.selection.position.min(length.saturating_sub(1));
                model
                    .home
                    .selection
                    .list
                    .select((length > 0).then_some(model.home.selection.position));
            }
        }
        for (feed_id, snapshot) in &mut model.home.feed_snapshots {
            if let Some(posts) = snapshot.timeline.as_mut() {
                for feed in posts.iter_mut() {
                    update(&mut feed.post.data);
                }
                if !bookmarked && feed_id == "bookmarks" {
                    posts.retain(|feed| feed.post.uri != uri);
                    snapshot.position = snapshot.position.min(posts.len().saturating_sub(1));
                }
            }
        }
        if let Some(posts) = model.explore.results.as_mut() {
            for post in posts {
                update(post);
            }
        }
        for post in model.notifications.posts.values_mut() {
            update(post);
        }
        if let Some(entries) = model.thread.entries.as_mut() {
            for entry in entries {
                if let ThreadEntry::Post { post, .. } = entry {
                    update(post);
                }
            }
        }
        if let Some(profile) = model.profile.as_mut() {
            if let ProfileContent::Posts(posts) = &mut profile.content {
                for feed in posts {
                    update(&mut feed.post.data);
                }
            }
        }
    }
}

mod feed_search;
mod input;
mod navigation;
#[path = "notifications.rs"]
mod notification_state;
mod pagination;
mod selection;
mod thread_profile;

fn select_by(position: &mut usize, state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        *position = 0;
        state.select(None);
        return;
    }
    *position = if delta == isize::MIN {
        0
    } else if delta == isize::MAX {
        len - 1
    } else if delta < 0 {
        position.saturating_sub(delta.unsigned_abs())
    } else {
        position.saturating_add(delta as usize).min(len - 1)
    };
    state.select(Some(*position));
}
