//! notifications Bluesky services.

use super::*;

pub async fn notifications(
    agent: &BskyAgent,
    cursor: Option<String>,
) -> Result<notification::list_notifications::Output> {
    let notifications = agent
        .api
        .app
        .bsky
        .notification
        .list_notifications(
            notification::list_notifications::ParametersData {
                cursor,
                limit: None,
                priority: None,
                reasons: None,
                seen_at: None,
            }
            .into(),
        )
        .await?;

    Ok(notifications)
}

pub async fn notification_posts(
    agent: &BskyAgent,
    notifications: &[notification::list_notifications::Notification],
) -> Result<Vec<defs::PostView>> {
    let mut uris = notifications
        .iter()
        .filter_map(crate::app::notifications::post_uri)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    uris.sort();
    uris.dedup();

    let mut posts = Vec::new();
    for chunk in uris.chunks(25) {
        let output = agent
            .api
            .app
            .bsky
            .feed
            .get_posts(
                get_posts::ParametersData {
                    uris: chunk.to_vec(),
                }
                .into(),
            )
            .await?;
        posts.extend(output.posts.iter().cloned());
    }
    Ok(posts)
}

pub async fn update_seen(agent: &BskyAgent) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .update_seen(
            notification::update_seen::InputData {
                seen_at: Datetime::now(),
            }
            .into(),
        )
        .await?;
    Ok(())
}

pub async fn notification_preferences(
    agent: &BskyAgent,
) -> Result<notification::defs::Preferences> {
    Ok(agent
        .api
        .app
        .bsky
        .notification
        .get_preferences(notification::get_preferences::ParametersData {}.into())
        .await?
        .preferences
        .clone())
}

pub async fn put_notification_preferences(
    agent: &BskyAgent,
    preferences: notification::defs::Preferences,
) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .put_preferences_v2(
            notification::put_preferences_v2::InputData {
                chat: Some(preferences.chat.clone()),
                follow: Some(preferences.follow.clone()),
                like: Some(preferences.like.clone()),
                like_via_repost: Some(preferences.like_via_repost.clone()),
                mention: Some(preferences.mention.clone()),
                quote: Some(preferences.quote.clone()),
                reply: Some(preferences.reply.clone()),
                repost: Some(preferences.repost.clone()),
                repost_via_repost: Some(preferences.repost_via_repost.clone()),
                starterpack_joined: Some(preferences.starterpack_joined.clone()),
                subscribed_post: Some(preferences.subscribed_post.clone()),
                unverified: Some(preferences.unverified.clone()),
                verified: Some(preferences.verified.clone()),
            }
            .into(),
        )
        .await?;
    Ok(())
}

pub async fn put_activity_subscription(
    agent: &BskyAgent,
    subject: Did,
    activity: notification::defs::ActivitySubscription,
) -> Result<()> {
    agent
        .api
        .app
        .bsky
        .notification
        .put_activity_subscription(
            notification::put_activity_subscription::InputData {
                activity_subscription: activity,
                subject,
            }
            .into(),
        )
        .await?;
    Ok(())
}
