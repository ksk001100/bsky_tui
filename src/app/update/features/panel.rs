use super::*;

impl App {
    pub(super) fn open_feature_target(&mut self, selected: Option<feature_panel::FeatureRow>) {
        match selected.map(|row| row.target) {
            Some(feature_panel::FeatureTarget::List { uri, .. }) => {
                self.dispatch(IoEvent::Feature(FeatureEvent::OpenList(uri)))
            }
            Some(feature_panel::FeatureTarget::StarterPack { uri, .. }) => {
                self.dispatch(IoEvent::Feature(FeatureEvent::OpenStarterPack(uri)))
            }
            Some(feature_panel::FeatureTarget::Conversation { id, .. }) => {
                self.dispatch(IoEvent::Feature(FeatureEvent::OpenConversation(id)))
            }
            Some(feature_panel::FeatureTarget::Labeler(did)) => {
                self.dispatch(IoEvent::Feature(FeatureEvent::OpenLabeler(did)))
            }
            Some(feature_panel::FeatureTarget::Actor(actor))
            | Some(feature_panel::FeatureTarget::ListMember { actor, .. }) => {
                self.feature_panel = None;
                self.dispatch(IoEvent::LoadProfile(actor));
            }
            Some(feature_panel::FeatureTarget::Topic(topic)) => {
                self.feature_panel = None;
                self.state.set_search_query(Some(topic.clone()));
                self.state.set_tab(Tab::Search);
                self.dispatch(IoEvent::Search(SearchEvent::Load(topic)));
            }
            Some(feature_panel::FeatureTarget::Account(identifier)) => {
                self.dispatch(IoEvent::Feature(FeatureEvent::SwitchAccount(identifier)));
            }
            Some(feature_panel::FeatureTarget::Setting(setting)) => self.open_feature_prompt(
                format!("Set {:?}", setting),
                "Enter the new value".into(),
                feature_panel::FeaturePromptAction::EditSetting(setting),
                String::new(),
            ),
            _ => {}
        }
    }

    pub(in crate::app) fn feature_prompt_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.prompt = None;
                }
            }
            Key::Enter => {
                let submitted = self.feature_panel.as_mut().and_then(|panel| {
                    panel
                        .prompt
                        .take()
                        .map(|prompt| (prompt.action, prompt.input.value().trim().to_owned()))
                });
                if let Some((action, value)) = submitted {
                    self.dispatch(IoEvent::Feature(FeatureEvent::Submit(action, value)));
                }
            }
            Key::Left | Key::Ctrl('b') => self.with_feature_input(InputRequest::GoToPrevChar),
            Key::Right | Key::Ctrl('f') => self.with_feature_input(InputRequest::GoToNextChar),
            Key::Ctrl('a') => self.with_feature_input(InputRequest::GoToStart),
            Key::Ctrl('e') => self.with_feature_input(InputRequest::GoToEnd),
            Key::Backspace | Key::Ctrl('h') => {
                self.with_feature_input(InputRequest::DeletePrevChar)
            }
            Key::Char(character) => self.with_feature_input(InputRequest::InsertChar(character)),
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn with_feature_input(&mut self, request: InputRequest) {
        if let Some(prompt) = self
            .feature_panel
            .as_mut()
            .and_then(|panel| panel.prompt.as_mut())
        {
            prompt.input.handle(request);
        }
    }

    pub(in crate::app) fn open_feature_prompt(
        &mut self,
        label: String,
        help: String,
        action: feature_panel::FeaturePromptAction,
        initial: String,
    ) {
        if let Some(panel) = self.feature_panel.as_mut() {
            panel.prompt = Some(feature_panel::FeaturePrompt {
                label,
                help,
                action,
                input: Input::new(initial),
            });
        }
    }

    pub fn set_feature_rows(
        &mut self,
        title: String,
        rows: Vec<feature_panel::FeatureRow>,
        child: bool,
    ) {
        if let Some(panel) = self.feature_panel.as_mut() {
            if child {
                *panel = panel.child(title, rows);
            } else {
                panel.replace(title, rows);
            }
        }
    }

    pub fn feature_panel(&self) -> Option<&feature_panel::FeaturePanel> {
        self.feature_panel.as_ref()
    }
}
