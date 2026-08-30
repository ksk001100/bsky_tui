use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_lists(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        use crate::app::feature_panel::FeatureTarget;

        let did = self.did().await?;
        let agent = self.agent().await?;
        let mut rows = bsky::feature_services::own_lists(agent.as_ref(), did.clone()).await?;
        let config = crate::app::config::AppConfig::load()?;
        for uri in config.saved_lists {
            if !rows.iter().any(|row| matches!(&row.target, FeatureTarget::List { uri: existing, .. } if existing == &uri)) {
                if let Ok(row) =
                    bsky::feature_services::list_overview(agent.as_ref(), uri, &did).await
                {
                    rows.push(row);
                }
            }
        }
        Ok(rows)
    }

    pub(super) async fn open_list(&mut self, uri: String) -> Result<()> {
        let did = self.did().await?;
        let rows =
            bsky::feature_services::list_detail(self.agent().await?.as_ref(), uri.clone(), did)
                .await?;
        self.emit(EffectMessage::FeatureRowsLoaded {
            title: format!("List · {uri}"),
            rows,
            child: true,
        })
        .await;
        Ok(())
    }

    pub(super) async fn create_list(&mut self, value: String) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 2).map_err(eyre::Report::msg)?;
        bsky::feature_services::create_list(
            self.agent().await?.as_ref(),
            fields[0].clone(),
            fields[1].clone(),
            fields.get(2).cloned().filter(|value| !value.is_empty()),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn edit_list(
        &mut self,
        uri: String,
        purpose: String,
        value: String,
    ) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 1).map_err(eyre::Report::msg)?;
        bsky::feature_services::edit_list(
            self.agent().await?.as_ref(),
            &uri,
            purpose,
            fields[0].clone(),
            fields.get(1).cloned().filter(|value| !value.is_empty()),
        )
        .await
    }

    pub(super) async fn add_list_member(&mut self, list_uri: String, value: String) -> Result<()> {
        let actor = value.parse().map_err(eyre::Report::msg)?;
        let agent = self.agent().await?;
        let profile = bsky::profile(agent.as_ref(), actor).await?;
        bsky::feature_services::add_list_member(agent.as_ref(), list_uri, profile.did.clone()).await
    }

    pub(super) async fn toggle_moderation_list(&mut self, uri: String, muted: bool) -> Result<()> {
        bsky::feature_services::toggle_moderation_list(self.agent().await?.as_ref(), uri, muted)
            .await?;
        self.load_feature_section(crate::app::feature_panel::FeatureSection::Lists)
            .await
    }

    pub(super) async fn use_list_feed(&mut self, uri: String, name: String) -> Result<()> {
        self.emit(EffectMessage::FeedActivated(
            crate::app::feed::FeedDescriptor::list(uri, name),
        ))
        .await;
        self.load_timeline(TimelineEvent::Load).await
    }

    pub(super) async fn save_list(&mut self, uri: String) -> Result<()> {
        let mut config = crate::app::config::AppConfig::load()?;
        if config.saved_lists.iter().any(|saved| saved == &uri) {
            config.saved_lists.retain(|saved| saved != &uri);
        } else {
            config.saved_lists.push(uri);
        }
        config.save()?;
        self.load_feature_section(crate::app::feature_panel::FeatureSection::Lists)
            .await
    }
}
