use super::*;

impl App {
    pub(super) fn lists_action(&mut self, key: Key, selected: Option<feature_panel::FeatureRow>) {
        match key {
            Key::Char('n') => self.open_feature_prompt(
                "Create list".into(),
                "curation|moderation | name | description".into(),
                feature_panel::FeaturePromptAction::CreateList,
                "curation | ".into(),
            ),
            Key::Char('a') => {
                if let Some(feature_panel::FeatureTarget::List {
                    uri, owned: true, ..
                }) = selected.map(|row| row.target)
                {
                    self.open_feature_prompt(
                        "Add list member".into(),
                        "Handle or DID".into(),
                        feature_panel::FeaturePromptAction::AddListMember { list_uri: uri },
                        String::new(),
                    );
                }
            }
            Key::Char('e') => {
                if let Some(feature_panel::FeatureTarget::List {
                    uri,
                    purpose,
                    owned: true,
                    ..
                }) = selected.map(|row| row.target)
                {
                    self.open_feature_prompt(
                        "Edit list".into(),
                        "name | description".into(),
                        feature_panel::FeaturePromptAction::EditList { uri, purpose },
                        String::new(),
                    );
                }
            }
            Key::Char('x') | Key::Delete => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List {
                    uri, owned: true, ..
                }) => self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(uri))),
                Some(feature_panel::FeatureTarget::ListMember { item_uri, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(item_uri)))
                }
                _ => {}
            },
            Key::Char('s') => {
                if let Some(feature_panel::FeatureTarget::List {
                    uri,
                    purpose,
                    muted,
                    ..
                }) = selected.map(|row| row.target)
                {
                    if purpose == atrium_api::app::bsky::graph::defs::MODLIST {
                        self.dispatch(IoEvent::Feature(FeatureEvent::ToggleModerationList {
                            uri,
                            muted,
                        }));
                    } else {
                        self.dispatch(IoEvent::Feature(FeatureEvent::SaveList(uri)));
                    }
                }
            }
            Key::Char('f') => {
                if let Some(row) = selected {
                    if let feature_panel::FeatureTarget::List { uri, purpose, .. } = row.target {
                        if purpose == atrium_api::app::bsky::graph::defs::CURATELIST {
                            self.feature_panel = None;
                            self.dispatch(IoEvent::Feature(FeatureEvent::UseListFeed {
                                uri,
                                name: row.title,
                            }));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
