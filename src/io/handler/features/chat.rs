use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_conversations(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        bsky::feature_services::conversations(self.agent().await?.as_ref(), &self.did().await?)
            .await
    }

    pub(super) async fn start_conversation(&mut self, value: String) -> Result<()> {
        let actor = value.parse().map_err(eyre::Report::msg)?;
        let agent = self.agent().await?;
        let profile = bsky::profile(agent.as_ref(), actor).await?;
        let convo_id =
            bsky::feature_services::start_conversation(agent.as_ref(), profile.did.clone()).await?;
        self.open_conversation(convo_id).await
    }

    pub(super) async fn send_message(&mut self, convo_id: String, value: String) -> Result<()> {
        bsky::feature_services::send_dm(self.agent().await?.as_ref(), convo_id.clone(), value)
            .await?;
        self.open_conversation(convo_id).await
    }

    pub(super) async fn toggle_conversation_mute(
        &mut self,
        convo_id: String,
        muted: bool,
    ) -> Result<()> {
        bsky::feature_services::toggle_conversation_mute(
            self.agent().await?.as_ref(),
            convo_id,
            muted,
        )
        .await?;
        self.load_feature_section(crate::app::feature_panel::FeatureSection::DirectMessages)
            .await
    }

    pub(super) async fn open_conversation(&mut self, convo_id: String) -> Result<()> {
        let rows =
            bsky::feature_services::conversation(self.agent().await?.as_ref(), convo_id.clone())
                .await?;
        self.emit(EffectMessage::ConversationLoaded {
            title: format!("Conversation · {convo_id}"),
            rows,
        })
        .await;
        Ok(())
    }
}
