//! social effects.

use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_thread(&mut self, uri: String) -> Result<()> {
        let output = bsky::post_thread(self.agent().await?.as_ref(), uri.clone()).await?;
        let entries = crate::app::thread::flatten(&output, &uri);
        let moderation = self.state().moderation();
        let image_urls = entries
            .iter()
            .filter_map(crate::app::thread::ThreadEntry::post)
            .flat_map(|post| bsky::post_image_urls(post, &moderation))
            .collect::<Vec<_>>();
        self.emit(EffectMessage::ThreadLoaded {
            entries,
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn load_interactions(
        &mut self,
        kind: InteractionKind,
        uri: String,
        cid: atrium_api::types::string::Cid,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let items = match kind {
            InteractionKind::Likes => bsky::post_likes(agent.as_ref(), uri, cid).await?,
            InteractionKind::Reposts => bsky::post_reposts(agent.as_ref(), uri, cid).await?,
            InteractionKind::Quotes => bsky::post_quotes(agent.as_ref(), uri, cid).await?,
            InteractionKind::Users | InteractionKind::Followers | InteractionKind::Follows => {
                return Err(eyre!("invalid post interaction type"));
            }
        };
        self.emit(EffectMessage::InteractionsLoaded { kind, items })
            .await;
        Ok(())
    }

    pub(super) async fn search_users(&mut self, query: String) -> Result<()> {
        let items = bsky::search_users(self.agent().await?.as_ref(), query).await?;
        self.emit(EffectMessage::UserSearchLoaded(items)).await;
        Ok(())
    }

    pub(super) async fn load_profile(
        &mut self,
        actor: atrium_api::types::string::AtIdentifier,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let details = bsky::profile(agent.as_ref(), actor).await?;
        let content = bsky::profile_content(
            agent.as_ref(),
            details.did.clone().into(),
            ProfileSection::Posts,
        )
        .await?;
        let moderation = self.state().moderation();
        let image_urls = vec![details.avatar.clone(), details.banner.clone()]
            .into_iter()
            .flatten()
            .chain(match &content {
                crate::app::profile::ProfileContent::Posts(posts) => posts
                    .iter()
                    .flat_map(|feed| bsky::post_image_urls(&feed.post, &moderation))
                    .collect::<Vec<_>>(),
                crate::app::profile::ProfileContent::Items(_) => Vec::new(),
            })
            .collect::<Vec<_>>();
        self.emit(EffectMessage::ProfileLoaded {
            profile: ProfileState::new(details, content),
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn load_profile_section(&mut self, section: ProfileSection) -> Result<()> {
        let actor = self
            .state()
            .get_profile()
            .map(|profile| profile.details.did.clone().into())
            .ok_or_else(|| eyre!("no profile is open"))?;
        let content = bsky::profile_content(self.agent().await?.as_ref(), actor, section).await?;
        let moderation = self.state().moderation();
        let image_urls = match &content {
            crate::app::profile::ProfileContent::Posts(posts) => posts
                .iter()
                .flat_map(|feed| bsky::post_image_urls(&feed.post, &moderation))
                .collect(),
            crate::app::profile::ProfileContent::Items(_) => Vec::new(),
        };
        self.emit(EffectMessage::ProfileContentLoaded {
            section,
            content,
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn toggle_follow(&mut self) -> Result<()> {
        let details = self
            .state()
            .get_profile()
            .map(|profile| profile.details)
            .ok_or_else(|| eyre!("no profile is open"))?;
        if Some(details.did.clone()) == self.state().get_did() {
            return Err(eyre!("you cannot follow your own account"));
        }
        let agent = self.agent().await?;
        let was_following = details
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.following.as_ref())
            .is_some();
        let following_uri = bsky::toggle_follow(agent.as_ref(), &details).await?;
        let mut refreshed = bsky::profile(agent.as_ref(), details.did.clone().into()).await?;
        refreshed.followers_count = Some(if was_following {
            details.followers_count.unwrap_or(1).saturating_sub(1)
        } else {
            details.followers_count.unwrap_or(0).saturating_add(1)
        });
        if let Some(viewer) = &mut refreshed.viewer {
            viewer.following = following_uri;
        }
        if let Some(mut profile) = self.state().get_profile() {
            profile.details = refreshed;
            self.emit(EffectMessage::ProfileUpdated(profile)).await;
        }
        Ok(())
    }

    pub(super) async fn load_connections(
        &mut self,
        kind: InteractionKind,
        actor: atrium_api::types::string::AtIdentifier,
    ) -> Result<()> {
        let agent = self.agent().await?;
        let items = match kind {
            InteractionKind::Followers => bsky::followers(agent.as_ref(), actor).await?,
            InteractionKind::Follows => bsky::follows(agent.as_ref(), actor).await?,
            _ => return Err(eyre!("invalid social graph list type")),
        };
        self.emit(EffectMessage::InteractionsLoaded { kind, items })
            .await;
        Ok(())
    }

    pub(super) async fn moderate(&mut self, action: crate::io::ModerationAction) -> Result<()> {
        let actor_change = match &action {
            crate::io::ModerationAction::MuteActor { did, .. }
            | crate::io::ModerationAction::BlockActor { did, .. } => Some(did.clone()),
            _ => None,
        };
        bsky::moderate(self.agent().await?.as_ref(), action).await?;
        if let Some(did) = actor_change {
            let mode = self.state().get_mode();
            let profile = self.state().get_profile();
            if let Some(mut profile) =
                profile.filter(|profile| mode == Mode::Profile && profile.details.did == did)
            {
                profile.details = bsky::profile(self.agent().await?.as_ref(), did.into()).await?;
                self.emit(EffectMessage::ProfileUpdated(profile)).await;
            } else {
                if mode == Mode::Thread {
                    self.emit(EffectMessage::ThreadClosed).await;
                }
                let tab = self.state().get_tab();
                if tab == Tab::Search {
                    self.search(SearchEvent::Reload).await?;
                } else if tab == Tab::Messages {
                    self.load_feature_section(
                        crate::app::feature_panel::FeatureSection::DirectMessages,
                    )
                    .await?;
                } else {
                    self.load_timeline(TimelineEvent::Reload).await?;
                }
            }
        }
        Ok(())
    }
}
