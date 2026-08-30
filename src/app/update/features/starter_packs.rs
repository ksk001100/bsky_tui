use super::*;

impl App {
    pub(super) fn starter_packs_action(
        &mut self,
        key: Key,
        selected: Option<feature_panel::FeatureRow>,
    ) {
        match key {
            Key::Char('n') => self.open_feature_prompt(
                "Create Starter Pack".into(),
                "name | description | list AT-URI".into(),
                feature_panel::FeaturePromptAction::CreateStarterPack,
                String::new(),
            ),
            Key::Char('e') => {
                if let Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) = selected.map(|row| row.target)
                {
                    self.open_feature_prompt(
                        "Edit Starter Pack".into(),
                        "name | description | list AT-URI".into(),
                        feature_panel::FeaturePromptAction::EditStarterPack { uri },
                        String::new(),
                    );
                }
            }
            Key::Char('x') | Key::Delete => {
                if let Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) = selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(uri)));
                }
            }
            Key::Char('J') => {
                let actors = self
                    .feature_panel
                    .as_ref()
                    .map(|panel| {
                        panel
                            .rows
                            .iter()
                            .filter_map(|row| match &row.target {
                                feature_panel::FeatureTarget::Actor(actor) => Some(actor.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if actors.is_empty() {
                    self.set_error("Open a Starter Pack before joining it".into());
                } else {
                    self.dispatch(IoEvent::Feature(FeatureEvent::JoinStarterPack(actors)));
                }
            }
            _ => {}
        }
    }
}
