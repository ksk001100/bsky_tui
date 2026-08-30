//! notifications effects.

use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_notifications(&mut self, event: NotificationEvent) -> Result<()> {
        let current = self.state().get_notifications_current_cursor_index();
        let cursors = self.state().get_notification_cursors();
        let target = match event {
            NotificationEvent::Load => 0,
            NotificationEvent::Reload => current,
            NotificationEvent::Prev if current > 0 => current - 1,
            NotificationEvent::Prev => return Ok(()),
            NotificationEvent::Next => current + 1,
        };
        let cursor = if matches!(event, NotificationEvent::Load) {
            None
        } else {
            cursors.get(target).cloned().flatten()
        };
        if matches!(event, NotificationEvent::Next) && cursor.is_none() {
            return Ok(());
        }
        let agent = self.agent().await?;
        let notifications = bsky::notifications(agent.as_ref(), cursor.clone()).await?;
        let hydrated_posts = bsky::notification_posts(agent.as_ref(), &notifications.notifications)
            .await
            .unwrap_or_default();
        let moderation = self.state().moderation();
        let image_urls = hydrated_posts
            .iter()
            .flat_map(|post| bsky::post_image_urls(post, &moderation))
            .collect::<Vec<_>>();
        let notification_posts = hydrated_posts
            .into_iter()
            .map(|post| (post.uri.clone(), post.data.clone()))
            .collect();
        bsky::update_seen(agent.as_ref()).await?;
        let page_cursors = updated_cursors(
            if matches!(event, NotificationEvent::Load) {
                vec![None]
            } else {
                cursors
            },
            target,
            cursor,
            notifications.cursor.clone(),
        );
        self.emit(EffectMessage::NotificationsLoaded {
            notifications: notifications.notifications.clone(),
            posts: notification_posts,
            cursors: page_cursors,
            cursor_index: target,
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn load_notification_settings(
        &mut self,
        subject: atrium_api::types::string::Did,
        handle: String,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let preferences = bsky::notification_preferences(agent.as_ref()).await?;
        let profile = bsky::profile(agent.as_ref(), subject.clone().into()).await?;
        let activity = profile
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.activity_subscription.clone())
            .unwrap_or_else(|| {
                atrium_api::app::bsky::notification::defs::ActivitySubscriptionData {
                    post: false,
                    reply: false,
                }
                .into()
            });
        self.emit(EffectMessage::NotificationSettingsLoaded(
            crate::app::notifications::NotificationSettings {
                preferences,
                category: 0,
                activity_subject: Some((subject, handle, activity)),
            },
        ))
        .await;
        Ok(())
    }

    pub(super) async fn toggle_notification_follow(
        &mut self,
        subject: atrium_api::types::string::Did,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let profile = bsky::profile(agent.as_ref(), subject.into()).await?;
        bsky::toggle_follow(agent.as_ref(), &profile).await?;
        self.load_notifications(NotificationEvent::Reload).await
    }

    pub(super) async fn like_notification_author(
        &mut self,
        subject: atrium_api::types::string::Did,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let content = bsky::profile_content(
            agent.as_ref(),
            subject.clone().into(),
            crate::app::profile::ProfileSection::Posts,
        )
        .await?;
        let post = match content {
            crate::app::profile::ProfileContent::Posts(posts) => posts
                .into_iter()
                .map(|item| item.post.clone())
                .find(|post| post.author.did == subject),
            _ => None,
        }
        .ok_or_else(|| eyre!("this account has no post to like"))?;
        if post
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.like.as_ref())
            .is_none()
        {
            bsky::like(
                agent.as_ref(),
                self.did().await?,
                post.cid.clone(),
                post.uri.clone(),
            )
            .await?;
        }
        Ok(())
    }

    pub(super) async fn save_notification_preferences(
        &self,
        preferences: atrium_api::app::bsky::notification::defs::Preferences,
    ) -> Result<()> {
        bsky::put_notification_preferences(self.agent().await?.as_ref(), preferences).await
    }

    pub(super) async fn save_activity_subscription(
        &self,
        subject: atrium_api::types::string::Did,
        activity: atrium_api::app::bsky::notification::defs::ActivitySubscription,
    ) -> Result<()> {
        bsky::put_activity_subscription(self.agent().await?.as_ref(), subject, activity).await
    }
}
