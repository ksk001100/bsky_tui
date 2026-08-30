//! composer effects.

use super::*;

impl IoAsyncHandler {
    pub(super) async fn send_post(&mut self) -> Result<()> {
        let text = self.state().input.clone().unwrap_or_default();
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(self.agent().await?.as_ref(), drafts, None).await?;
        self.finish_composer().await;
        self.load_timeline(TimelineEvent::Reload).await
    }

    pub(super) async fn search(&mut self, event: SearchEvent) -> Result<()> {
        let current = self.state().get_search_current_cursor_index();
        let cursors = self.state().get_search_cursors();
        let existing_query = self.state().get_search_query();
        let target = match &event {
            SearchEvent::Load(_) => 0,
            SearchEvent::Reload => current,
            SearchEvent::Prev if current > 0 => current - 1,
            SearchEvent::Prev => return Ok(()),
            SearchEvent::Next => current + 1,
        };
        let cursor = if matches!(event, SearchEvent::Load(_)) {
            None
        } else {
            cursors.get(target).cloned().flatten()
        };
        if matches!(event, SearchEvent::Next) && cursor.is_none() {
            return Ok(());
        }
        let query = match &event {
            SearchEvent::Load(query) => query.clone(),
            _ => existing_query.ok_or_else(|| eyre!("no search query is active"))?,
        };

        let results =
            bsky::search(self.agent().await?.as_ref(), query.clone(), cursor.clone()).await?;
        let moderation = self.state().moderation();
        let image_urls = results
            .posts
            .iter()
            .flat_map(|post| bsky::post_image_urls(post, &moderation))
            .collect::<Vec<_>>();
        let page_cursors = updated_cursors(
            if matches!(event, SearchEvent::Load(_)) {
                vec![None]
            } else {
                cursors
            },
            target,
            cursor,
            results.cursor.clone(),
        );

        self.emit(EffectMessage::SearchLoaded {
            query,
            posts: results.posts.iter().map(|post| post.data.clone()).collect(),
            cursors: page_cursors,
            cursor_index: target,
            image_urls,
        })
        .await;
        Ok(())
    }

    pub(super) async fn like(&mut self) -> Result<()> {
        let did = self
            .state()
            .get_did()
            .ok_or_else(|| eyre!("account is not initialized"))?;
        let feed = self
            .state()
            .get_current_feed()
            .ok_or_else(|| eyre!("no timeline post is selected"))?;
        bsky::toggle_like(self.agent().await?.as_ref(), did, feed).await?;
        self.load_timeline(TimelineEvent::Reload).await
    }

    pub(super) async fn repost(&mut self) -> Result<()> {
        let did = self
            .state()
            .get_did()
            .ok_or_else(|| eyre!("account is not initialized"))?;
        let feed = self
            .state()
            .get_current_feed()
            .ok_or_else(|| eyre!("no timeline post is selected"))?;
        bsky::toggle_repost(self.agent().await?.as_ref(), did, feed).await?;
        self.load_timeline(TimelineEvent::Reload).await
    }

    pub(super) async fn reply(&mut self) -> Result<()> {
        let feed = self
            .state()
            .get_current_feed()
            .ok_or_else(|| eyre!("no timeline post is selected"))?;
        let text = self.state().input.clone().unwrap_or_default();
        let parent = strong_ref::MainData {
            cid: feed.post.cid.clone(),
            uri: feed.post.uri.clone(),
        };
        let root = bsky::reply_root(&feed).unwrap_or_else(|| parent.clone());
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(
            self.agent().await?.as_ref(),
            drafts,
            Some(
                ReplyRefData {
                    root: root.into(),
                    parent: parent.into(),
                }
                .into(),
            ),
        )
        .await?;
        self.finish_composer().await;
        self.load_timeline(TimelineEvent::Reload).await
    }

    pub(super) async fn search_like(&mut self) -> Result<()> {
        let (did, post) = self.selected_search_post().await?;
        bsky::toggle_like_post_view(self.agent().await?.as_ref(), did, post).await?;
        self.search(SearchEvent::Reload).await
    }

    pub(super) async fn search_repost(&mut self) -> Result<()> {
        let (did, post) = self.selected_search_post().await?;
        bsky::toggle_repost_post_view(self.agent().await?.as_ref(), did, post).await?;
        self.search(SearchEvent::Reload).await
    }

    pub(super) async fn selected_search_post(
        &self,
    ) -> Result<(
        atrium_api::types::string::Did,
        atrium_api::app::bsky::feed::defs::PostViewData,
    )> {
        Ok((
            self.state()
                .get_did()
                .ok_or_else(|| eyre!("account is not initialized"))?,
            self.state()
                .get_current_search_result()
                .ok_or_else(|| eyre!("no search result is selected"))?,
        ))
    }

    pub(super) async fn search_reply(&mut self) -> Result<()> {
        let (_, post) = self.selected_search_post().await?;
        let text = self.state().input.clone().unwrap_or_default();
        let subject = strong_ref::MainData {
            cid: post.cid.clone(),
            uri: post.uri.clone(),
        };
        let root = bsky::reply_root_for_post(self.agent().await?.as_ref(), &post).await?;
        let drafts = drafts_with_default_language(&text)?;
        bsky::send_drafts(
            self.agent().await?.as_ref(),
            drafts,
            Some(
                ReplyRefData {
                    root: root.into(),
                    parent: subject.into(),
                }
                .into(),
            ),
        )
        .await?;
        self.finish_composer().await;
        self.search(SearchEvent::Reload).await
    }

    pub(super) async fn finish_composer(&mut self) {
        self.emit(EffectMessage::ComposerFinished).await;
    }
}
