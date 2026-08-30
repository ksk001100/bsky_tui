use super::*;

impl App {
    pub(super) fn open_direct_messages(&mut self) {
        self.feature_panel = None;
        self.state.set_tab(Tab::Messages);
        self.dispatch(IoEvent::Feature(FeatureEvent::Load(
            feature_panel::FeatureSection::DirectMessages,
        )));
    }

    pub(super) fn chat_action(&mut self, key: Key, selected: Option<feature_panel::FeatureRow>) {
        match key {
            Key::Char('n') => self.open_feature_prompt(
                "New conversation".into(),
                "Handle or DID".into(),
                feature_panel::FeaturePromptAction::NewConversation,
                String::new(),
            ),
            Key::Char(' ') => {
                if let Some(feature_panel::FeatureTarget::Conversation { id, muted, .. }) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleConversationMute {
                        convo_id: id,
                        muted,
                    }));
                }
            }
            Key::Char('w') => {
                let convo_id = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Conversation { id, .. }) => Some(id),
                    Some(feature_panel::FeatureTarget::Message { convo_id, .. }) => Some(convo_id),
                    _ => None,
                }
                .or_else(|| {
                    self.feature_panel.as_ref().and_then(|panel| {
                        panel
                            .title
                            .strip_prefix("Conversation · ")
                            .map(str::to_owned)
                    })
                });
                if let Some(convo_id) = convo_id {
                    self.open_feature_prompt(
                        "Send message".into(),
                        "Text message (1000 characters maximum)".into(),
                        feature_panel::FeaturePromptAction::SendMessage { convo_id },
                        String::new(),
                    );
                }
            }
            _ => {}
        }
    }

    pub fn set_message_rows(&mut self, title: String, rows: Vec<feature_panel::FeatureRow>) {
        self.messages.replace(title, rows);
    }

    pub fn open_message_conversation(
        &mut self,
        title: String,
        rows: Vec<feature_panel::FeatureRow>,
    ) {
        if self.messages.title.starts_with("Conversation ·") {
            self.messages.replace(title, rows);
        } else {
            self.messages = self.messages.child(title, rows);
        }
    }

    pub fn messages(&self) -> &feature_panel::FeaturePanel {
        &self.messages
    }
}
