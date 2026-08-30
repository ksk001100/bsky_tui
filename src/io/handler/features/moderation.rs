use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_moderation(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        use crate::app::feature_panel::{FeatureRow, FeatureTarget};

        // Loading every feature section previously verified the initialized account first.
        self.did().await?;
        let mut rows =
            bsky::feature_services::moderation_preferences_rows(self.agent().await?.as_ref())
                .await?;
        let config = crate::app::config::AppConfig::load()?;
        rows.extend(config.muted_threads.into_iter().map(|uri| FeatureRow {
            title: "Muted thread".into(),
            detail: uri.clone(),
            target: FeatureTarget::MutedThread(uri),
            unread: false,
        }));
        Ok(rows)
    }

    pub(super) async fn open_labeler(&mut self, did: atrium_api::types::string::Did) -> Result<()> {
        let rows =
            bsky::feature_services::labeler_detail(self.agent().await?.as_ref(), did.clone())
                .await?;
        self.emit(EffectMessage::FeatureRowsLoaded {
            title: format!("Labeler · {}", did.as_str()),
            rows,
            child: true,
        })
        .await;
        Ok(())
    }

    pub(super) async fn add_muted_word(&mut self, value: String) -> Result<()> {
        bsky::feature_services::add_muted_word(self.agent().await?.as_ref(), value).await
    }

    pub(super) async fn add_labeler(&mut self, value: String) -> Result<()> {
        let did = atrium_api::types::string::Did::new(value).map_err(eyre::Report::msg)?;
        bsky::feature_services::toggle_labeler(self.agent().await?.as_ref(), did).await
    }

    pub(super) async fn set_label_visibility(
        &mut self,
        labeler: atrium_api::types::string::Did,
        label: String,
        value: String,
    ) -> Result<()> {
        bsky::feature_services::set_label_visibility(
            self.agent().await?.as_ref(),
            Some(labeler),
            label,
            value,
        )
        .await
    }

    pub(super) async fn report(
        &mut self,
        subject: crate::app::feature_panel::ReportSubject,
        value: String,
    ) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 1).map_err(eyre::Report::msg)?;
        bsky::feature_services::report(
            self.agent().await?.as_ref(),
            subject,
            &fields[0],
            fields.get(1).cloned(),
        )
        .await
    }

    pub(super) async fn remove_muted_word(&mut self, word: String) -> Result<()> {
        bsky::feature_services::remove_muted_word(self.agent().await?.as_ref(), &word).await?;
        self.load_feature_section(crate::app::feature_panel::FeatureSection::Moderation)
            .await
    }

    pub(super) async fn toggle_labeler(
        &mut self,
        did: atrium_api::types::string::Did,
    ) -> Result<()> {
        bsky::feature_services::toggle_labeler(self.agent().await?.as_ref(), did).await?;
        self.load_feature_section(crate::app::feature_panel::FeatureSection::Moderation)
            .await
    }

    pub(super) async fn toggle_thread_mute(&mut self, root: String, muted: bool) -> Result<()> {
        bsky::feature_services::toggle_thread_mute(
            self.agent().await?.as_ref(),
            root.clone(),
            muted,
        )
        .await?;
        let mut config = crate::app::config::AppConfig::load()?;
        if muted {
            config.muted_threads.retain(|uri| uri != &root);
        } else if !config.muted_threads.contains(&root) {
            config.muted_threads.push(root);
        }
        config.save()?;
        if self
            .context
            .as_ref()
            .is_some_and(|context| context.feature_panel_open)
        {
            self.load_feature_section(crate::app::feature_panel::FeatureSection::Moderation)
                .await
        } else {
            Ok(())
        }
    }

    pub(super) async fn toggle_hidden_reply(
        &mut self,
        root: Box<atrium_api::app::bsky::feed::defs::PostViewData>,
        reply: String,
    ) -> Result<()> {
        let uri = root.uri.clone();
        bsky::feature_services::toggle_hidden_reply(self.agent().await?.as_ref(), &root, reply)
            .await?;
        self.load_thread(uri).await
    }

    pub(super) async fn detach_quote(&mut self, post: String, quote: String) -> Result<()> {
        bsky::feature_services::detach_quote(self.agent().await?.as_ref(), post, quote.clone())
            .await?;
        self.load_thread(quote).await
    }
}
