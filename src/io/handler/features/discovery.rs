use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_discovery(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        bsky::feature_services::discovery(self.agent().await?.as_ref(), self.did().await?).await
    }
}
