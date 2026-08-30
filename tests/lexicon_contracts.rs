use atrium_api::{
    app::bsky::{
        actor::defs::{ContentLabelPrefData, ProfileViewDetailedData},
        bookmark::get_bookmarks,
        feed::defs::PostViewData,
        notification::list_notifications::NotificationData,
    },
    chat::bsky::convo::defs::ConvoViewData,
    types::Union,
};

fn post_json(embed: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "author": {
            "did": "did:plc:alice",
            "handle": "alice.test"
        },
        "cid": "bafyreigh2akiscaildc4trlswnrrrw6qb4ra2n3a5pquusny6cf6om7y4e",
        "embed": embed,
        "indexedAt": "2026-08-30T00:00:00.000Z",
        "record": {
            "$type": "app.bsky.feed.post",
            "createdAt": "2026-08-30T00:00:00.000Z",
            "text": "contract fixture"
        },
        "uri": "at://did:plc:alice/app.bsky.feed.post/example",
        "viewer": { "bookmarked": true }
    })
}

#[test]
fn post_contract_preserves_bookmark_viewer_state() {
    let post: PostViewData =
        serde_json::from_value(post_json(serde_json::Value::Null)).expect("post fixture");

    assert_eq!(post.viewer.and_then(|viewer| viewer.bookmarked), Some(true));
}

#[test]
fn unknown_embed_is_rendered_as_unsupported_content() {
    let post: PostViewData = serde_json::from_value(post_json(serde_json::json!({
        "$type": "app.bsky.embed.futureMedia#view",
        "payload": "new lexicon data"
    })))
    .expect("post fixture with an open union");

    assert!(matches!(post.embed, Some(Union::Unknown(_))));
    assert_eq!(
        bsky_tui::bsky::post_embed_lines(&post),
        ["[Unsupported content]"]
    );
}

#[test]
fn bookmarks_contract_keeps_cursor_and_unknown_items() {
    let output: get_bookmarks::OutputData = serde_json::from_value(serde_json::json!({
        "bookmarks": [{
            "item": {
                "$type": "app.bsky.feed.defs#futurePostView",
                "uri": "at://did:plc:alice/app.bsky.feed.post/future"
            },
            "subject": {
                "cid": "bafyreigh2akiscaildc4trlswnrrrw6qb4ra2n3a5pquusny6cf6om7y4e",
                "uri": "at://did:plc:alice/app.bsky.feed.post/future"
            }
        }],
        "cursor": "next-page"
    }))
    .expect("bookmark fixture with an open union");

    assert_eq!(output.cursor.as_deref(), Some("next-page"));
    assert!(matches!(output.bookmarks[0].item, Union::Unknown(_)));
}

#[test]
fn notification_profile_dm_and_moderation_contracts_deserialize() {
    let notification: NotificationData = serde_json::from_value(serde_json::json!({
        "author": { "did": "did:plc:bob", "handle": "bob.test" },
        "cid": "bafyreigh2akiscaildc4trlswnrrrw6qb4ra2n3a5pquusny6cf6om7y4e",
        "indexedAt": "2026-08-30T00:00:00.000Z",
        "isRead": false,
        "reason": "mention",
        "record": { "$type": "app.bsky.feed.post", "text": "hello" },
        "uri": "at://did:plc:bob/app.bsky.feed.post/example"
    }))
    .expect("notification contract");
    assert_eq!(notification.reason, "mention");

    let profile: ProfileViewDetailedData = serde_json::from_value(serde_json::json!({
        "did": "did:plc:bob",
        "displayName": "Bob",
        "handle": "bob.test",
        "viewer": { "muted": false }
    }))
    .expect("profile contract");
    assert_eq!(profile.display_name.as_deref(), Some("Bob"));

    let conversation: ConvoViewData = serde_json::from_value(serde_json::json!({
        "id": "convo-1",
        "members": [{ "did": "did:plc:bob", "handle": "bob.test" }],
        "muted": false,
        "rev": "1",
        "unreadCount": 1
    }))
    .expect("DM contract");
    assert_eq!(conversation.members.len(), 1);

    let moderation: ContentLabelPrefData = serde_json::from_value(serde_json::json!({
        "label": "graphic-media",
        "labelerDid": "did:plc:labeler",
        "visibility": "warn"
    }))
    .expect("moderation contract");
    assert_eq!(moderation.visibility, "warn");
}
