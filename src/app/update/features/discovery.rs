use super::*;

impl App {
    pub(super) fn open_discovery(&mut self) {
        self.feature_panel = None;
        self.state.set_tab(Tab::Search);
        self.state.set_search_query(None);
        self.dispatch(IoEvent::Feature(FeatureEvent::Load(
            feature_panel::FeatureSection::Discovery,
        )));
    }

    pub fn set_explore_rows(&mut self, rows: Vec<feature_panel::FeatureRow>) {
        self.explore.replace(
            feature_panel::FeatureSection::Discovery.title().to_owned(),
            rows,
        );
    }

    pub fn explore(&self) -> &feature_panel::FeaturePanel {
        &self.explore
    }
}
