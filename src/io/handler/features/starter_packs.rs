use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_starter_packs(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        bsky::feature_services::starter_packs(self.agent().await?.as_ref(), self.did().await?).await
    }

    pub(super) async fn open_starter_pack(&mut self, uri: String) -> Result<()> {
        let rows =
            bsky::feature_services::starter_pack_detail(self.agent().await?.as_ref(), uri.clone())
                .await?;
        self.emit(EffectMessage::FeatureRowsLoaded {
            title: format!("Starter Pack · {uri}"),
            rows,
            child: true,
        })
        .await;
        Ok(())
    }

    pub(super) async fn create_starter_pack(&mut self, value: String) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 3).map_err(eyre::Report::msg)?;
        bsky::feature_services::create_starter_pack(
            self.agent().await?.as_ref(),
            fields[0].clone(),
            Some(fields[1].clone()).filter(|value| !value.is_empty()),
            fields[2].clone(),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn edit_starter_pack(&mut self, uri: String, value: String) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 3).map_err(eyre::Report::msg)?;
        bsky::feature_services::edit_starter_pack(
            self.agent().await?.as_ref(),
            &uri,
            fields[0].clone(),
            Some(fields[1].clone()).filter(|value| !value.is_empty()),
            fields[2].clone(),
        )
        .await
    }

    pub(super) async fn join_starter_pack(
        &mut self,
        actors: Vec<atrium_api::types::string::AtIdentifier>,
    ) -> Result<()> {
        let agent = self.agent().await?;
        for actor in actors {
            let profile = bsky::profile(agent.as_ref(), actor).await?;
            if profile
                .viewer
                .as_ref()
                .and_then(|viewer| viewer.following.as_ref())
                .is_none()
            {
                bsky::toggle_follow(agent.as_ref(), &profile).await?;
            }
        }
        Ok(())
    }
}
