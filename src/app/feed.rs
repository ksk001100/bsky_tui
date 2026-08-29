#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedKind {
    Following,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: FeedKind,
    pub saved: bool,
    pub pinned: bool,
}

impl FeedDescriptor {
    pub fn following() -> Self {
        Self {
            id: "following".to_owned(),
            name: "Following".to_owned(),
            description: "Posts from accounts you follow".to_owned(),
            kind: FeedKind::Following,
            saved: true,
            pinned: true,
        }
    }

    pub fn discover() -> Self {
        let uri = "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot";
        Self {
            id: uri.to_owned(),
            name: "Discover".to_owned(),
            description: "Popular posts selected by Bluesky".to_owned(),
            kind: FeedKind::Custom(uri.to_owned()),
            saved: false,
            pinned: false,
        }
    }
}

#[derive(Clone)]
pub struct FeedSnapshot {
    pub timeline: Option<Vec<atrium_api::app::bsky::feed::defs::FeedViewPost>>,
    pub position: usize,
    pub page: usize,
    pub cursors: Vec<Option<String>>,
    pub new_count: usize,
}

impl Default for FeedSnapshot {
    fn default() -> Self {
        Self {
            timeline: None,
            position: 0,
            page: 0,
            cursors: vec![None],
            new_count: 0,
        }
    }
}
