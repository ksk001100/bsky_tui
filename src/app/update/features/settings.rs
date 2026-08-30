use super::*;

impl App {
    pub(super) fn settings_action(
        &mut self,
        key: Key,
        _selected: Option<feature_panel::FeatureRow>,
    ) {
        if key == Key::Char('n') {
            self.open_feature_prompt(
                "Add account".into(),
                "identifier | service URL (save credentials separately)".into(),
                feature_panel::FeaturePromptAction::AddAccount,
                String::new(),
            );
        }
    }
}
