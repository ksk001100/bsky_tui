use atrium_api::{
    app::bsky::feed::{defs, get_post_thread},
    types::Union,
};

#[derive(Clone)]
pub enum ThreadEntry {
    Post {
        post: Box<defs::PostViewData>,
        depth: usize,
        target: bool,
    },
    Placeholder {
        message: String,
        depth: usize,
    },
}

impl ThreadEntry {
    pub fn post(&self) -> Option<&defs::PostViewData> {
        match self {
            Self::Post { post, .. } => Some(post.as_ref()),
            Self::Placeholder { .. } => None,
        }
    }
}

pub fn flatten(output: &get_post_thread::Output, target_uri: &str) -> Vec<ThreadEntry> {
    let mut entries = Vec::new();
    match &output.thread {
        Union::Refs(get_post_thread::OutputThreadRefs::AppBskyFeedDefsThreadViewPost(thread)) => {
            push_parents(thread, target_uri, &mut entries);
            push_post_and_replies(thread, 0, target_uri, &mut entries);
        }
        Union::Refs(get_post_thread::OutputThreadRefs::AppBskyFeedDefsNotFoundPost(_)) => {
            entries.push(ThreadEntry::Placeholder {
                message: "[Deleted or not-found post]".to_owned(),
                depth: 0,
            });
        }
        Union::Refs(get_post_thread::OutputThreadRefs::AppBskyFeedDefsBlockedPost(_)) => {
            entries.push(ThreadEntry::Placeholder {
                message: "[Post unavailable: blocked author]".to_owned(),
                depth: 0,
            });
        }
        Union::Unknown(_) => entries.push(ThreadEntry::Placeholder {
            message: "[Unsupported thread node]".to_owned(),
            depth: 0,
        }),
    }
    entries
}

fn push_parents(thread: &defs::ThreadViewPost, target_uri: &str, entries: &mut Vec<ThreadEntry>) {
    let Some(parent) = &thread.parent else {
        return;
    };
    match parent {
        Union::Refs(defs::ThreadViewPostParentRefs::ThreadViewPost(parent)) => {
            push_parents(parent, target_uri, entries);
            entries.push(ThreadEntry::Post {
                post: Box::new(parent.post.data.clone()),
                depth: 0,
                target: parent.post.uri == target_uri,
            });
        }
        Union::Refs(defs::ThreadViewPostParentRefs::NotFoundPost(_)) => {
            entries.push(ThreadEntry::Placeholder {
                message: "[Deleted or not-found parent post]".to_owned(),
                depth: 0,
            });
        }
        Union::Refs(defs::ThreadViewPostParentRefs::BlockedPost(_)) => {
            entries.push(ThreadEntry::Placeholder {
                message: "[Parent post unavailable: blocked author]".to_owned(),
                depth: 0,
            });
        }
        Union::Unknown(_) => entries.push(ThreadEntry::Placeholder {
            message: "[Unsupported parent post]".to_owned(),
            depth: 0,
        }),
    }
}

fn push_post_and_replies(
    thread: &defs::ThreadViewPost,
    depth: usize,
    target_uri: &str,
    entries: &mut Vec<ThreadEntry>,
) {
    entries.push(ThreadEntry::Post {
        post: Box::new(thread.post.data.clone()),
        depth,
        target: thread.post.uri == target_uri,
    });
    for reply in thread.replies.iter().flatten() {
        match reply {
            Union::Refs(defs::ThreadViewPostRepliesItem::ThreadViewPost(reply)) => {
                push_post_and_replies(reply, depth + 1, target_uri, entries);
            }
            Union::Refs(defs::ThreadViewPostRepliesItem::NotFoundPost(_)) => {
                entries.push(ThreadEntry::Placeholder {
                    message: "[Deleted or not-found reply]".to_owned(),
                    depth: depth + 1,
                });
            }
            Union::Refs(defs::ThreadViewPostRepliesItem::BlockedPost(_)) => {
                entries.push(ThreadEntry::Placeholder {
                    message: "[Reply unavailable: blocked author]".to_owned(),
                    depth: depth + 1,
                });
            }
            Union::Unknown(_) => entries.push(ThreadEntry::Placeholder {
                message: "[Unsupported reply]".to_owned(),
                depth: depth + 1,
            }),
        }
    }
}
