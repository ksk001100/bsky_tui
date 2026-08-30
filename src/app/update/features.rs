//! features update handlers.

use super::super::*;

impl App {
    pub(in crate::app) fn feature_panel_action(&mut self, key: Key) -> AppReturn {
        if self
            .feature_panel
            .as_ref()
            .and_then(|panel| panel.prompt.as_ref())
            .is_some()
        {
            return self.feature_prompt_action(key);
        }

        let section = match key {
            Key::Char('1') => Some(feature_panel::FeatureSection::Lists),
            Key::Char('2') => Some(feature_panel::FeatureSection::StarterPacks),
            Key::Char('3') => {
                self.feature_panel = None;
                self.state.set_tab(Tab::Search);
                self.state.set_search_query(None);
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::Discovery,
                )));
                return AppReturn::Continue;
            }
            Key::Char('4') => {
                self.feature_panel = None;
                self.state.set_tab(Tab::Messages);
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::DirectMessages,
                )));
                return AppReturn::Continue;
            }
            Key::Char('5') => Some(feature_panel::FeatureSection::Moderation),
            Key::Char('6') => Some(feature_panel::FeatureSection::Settings),
            _ => None,
        };
        if let Some(section) = section {
            self.feature_panel = Some(feature_panel::FeaturePanel::loading(section));
            self.dispatch(IoEvent::Feature(FeatureEvent::Load(section)));
            return AppReturn::Continue;
        }

        let selected = self
            .feature_panel
            .as_ref()
            .and_then(feature_panel::FeaturePanel::selected_row)
            .cloned();
        match key {
            Key::Esc => {
                let parent = self
                    .feature_panel
                    .as_mut()
                    .and_then(|panel| panel.parent.take())
                    .map(|parent| *parent);
                if let Some(parent) = parent {
                    self.feature_panel = Some(parent);
                } else {
                    self.feature_panel = None;
                }
            }
            Key::Char('k') | Key::Up => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.previous();
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.next();
                }
            }
            Key::Enter | Key::Char('o') => match selected.map(|row| row.target) {
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
                Some(feature_panel::FeatureTarget::Setting(setting)) => {
                    self.open_feature_prompt(
                        format!("Set {:?}", setting),
                        "Enter the new value".into(),
                        feature_panel::FeaturePromptAction::EditSetting(setting),
                        String::new(),
                    );
                }
                _ => {}
            },
            Key::Char('n') => {
                let section = self.feature_panel.as_ref().map(|panel| panel.section);
                match section {
                    Some(feature_panel::FeatureSection::Lists) => self.open_feature_prompt(
                        "Create list".into(),
                        "curation|moderation | name | description".into(),
                        feature_panel::FeaturePromptAction::CreateList,
                        "curation | ".into(),
                    ),
                    Some(feature_panel::FeatureSection::StarterPacks) => self.open_feature_prompt(
                        "Create Starter Pack".into(),
                        "name | description | list AT-URI".into(),
                        feature_panel::FeaturePromptAction::CreateStarterPack,
                        String::new(),
                    ),
                    Some(feature_panel::FeatureSection::DirectMessages) => self
                        .open_feature_prompt(
                            "New conversation".into(),
                            "Handle or DID".into(),
                            feature_panel::FeaturePromptAction::NewConversation,
                            String::new(),
                        ),
                    Some(feature_panel::FeatureSection::Moderation) => self.open_feature_prompt(
                        "Mute word".into(),
                        "Word or phrase".into(),
                        feature_panel::FeaturePromptAction::AddMutedWord,
                        String::new(),
                    ),
                    Some(feature_panel::FeatureSection::Settings) => self.open_feature_prompt(
                        "Add account".into(),
                        "identifier | service URL (save credentials separately)".into(),
                        feature_panel::FeaturePromptAction::AddAccount,
                        String::new(),
                    ),
                    _ => {}
                }
            }
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
            Key::Char('e') => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List {
                    uri,
                    purpose,
                    owned: true,
                    ..
                }) => self.open_feature_prompt(
                    "Edit list".into(),
                    "name | description".into(),
                    feature_panel::FeaturePromptAction::EditList { uri, purpose },
                    String::new(),
                ),
                Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) => self.open_feature_prompt(
                    "Edit Starter Pack".into(),
                    "name | description | list AT-URI".into(),
                    feature_panel::FeaturePromptAction::EditStarterPack { uri },
                    String::new(),
                ),
                Some(feature_panel::FeatureTarget::LabelSetting { labeler, label }) => self
                    .open_feature_prompt(
                        format!("Label visibility · {label}"),
                        "ignore / warn / hide".into(),
                        feature_panel::FeaturePromptAction::SetLabelVisibility { labeler, label },
                        "warn".into(),
                    ),
                _ => {}
            },
            Key::Char('x') | Key::Delete => match selected.map(|row| row.target) {
                Some(feature_panel::FeatureTarget::List {
                    uri, owned: true, ..
                })
                | Some(feature_panel::FeatureTarget::StarterPack {
                    uri, owned: true, ..
                }) => self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(uri))),
                Some(feature_panel::FeatureTarget::ListMember { item_uri, .. }) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::DeleteRecord(item_uri)))
                }
                Some(feature_panel::FeatureTarget::MutedWord(word)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::RemoveMutedWord(word)))
                }
                Some(feature_panel::FeatureTarget::MutedThread(root)) => {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleThreadMute {
                        root,
                        muted: true,
                    }))
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
            Key::Char('l') => {
                if let Some(feature_panel::FeatureTarget::Labeler(did)) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleLabeler(did)));
                }
            }
            Key::Char('L') => {
                if self
                    .feature_panel
                    .as_ref()
                    .is_some_and(|panel| panel.section == feature_panel::FeatureSection::Moderation)
                {
                    self.open_feature_prompt(
                        "Subscribe to labeler".into(),
                        "Labeler DID".into(),
                        feature_panel::FeaturePromptAction::AddLabeler,
                        String::new(),
                    );
                }
            }
            Key::Char('J') => {
                if self.feature_panel.as_ref().is_some_and(|panel| {
                    panel.section == feature_panel::FeatureSection::StarterPacks
                }) {
                    let actors = self
                        .feature_panel
                        .as_ref()
                        .map(|panel| {
                            panel
                                .rows
                                .iter()
                                .filter_map(|row| match &row.target {
                                    feature_panel::FeatureTarget::Actor(actor) => {
                                        Some(actor.clone())
                                    }
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
            }
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
            Key::Char('r') => {
                let subject = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Actor(AtIdentifier::Did(did)))
                    | Some(feature_panel::FeatureTarget::ListMember {
                        actor: AtIdentifier::Did(did),
                        ..
                    }) => Some(feature_panel::ReportSubject::Account(did)),
                    Some(feature_panel::FeatureTarget::Message {
                        convo_id,
                        id,
                        sender,
                    }) => Some(feature_panel::ReportSubject::Conversation {
                        convo_id,
                        message_id: Some(format!("{id}@{}", sender.as_str())),
                        sender,
                    }),
                    Some(feature_panel::FeatureTarget::List { uri, cid, .. })
                    | Some(feature_panel::FeatureTarget::StarterPack { uri, cid, .. }) => {
                        Some(feature_panel::ReportSubject::Record { uri, cid })
                    }
                    Some(feature_panel::FeatureTarget::Conversation { id, members, .. }) => members
                        .into_iter()
                        .next()
                        .map(|sender| feature_panel::ReportSubject::Conversation {
                            convo_id: id,
                            message_id: None,
                            sender,
                        }),
                    _ => None,
                };
                if let Some(subject) = subject {
                    self.open_feature_prompt(
                        "Report".into(),
                        "spam|rude|sexual|violation|misleading|other | details".into(),
                        feature_panel::FeaturePromptAction::Report { subject },
                        "other | ".into(),
                    );
                }
            }
            Key::Char('b') => {
                let did = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Actor(AtIdentifier::Did(did)))
                    | Some(feature_panel::FeatureTarget::ListMember {
                        actor: AtIdentifier::Did(did),
                        ..
                    }) => Some(did),
                    Some(feature_panel::FeatureTarget::Conversation { members, .. }) => {
                        members.into_iter().next()
                    }
                    Some(feature_panel::FeatureTarget::Message { sender, .. }) => Some(sender),
                    _ => None,
                };
                if let Some(did) = did {
                    self.pending_confirmation = Some(ModerationAction::BlockActor {
                        did,
                        blocking_uri: None,
                    });
                }
            }
            _ => {}
        }
        AppReturn::Continue
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
            Key::Char(character) => {
                self.with_feature_input(InputRequest::InsertChar(character));
            }
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

    pub fn set_explore_rows(&mut self, rows: Vec<feature_panel::FeatureRow>) {
        self.explore.replace(
            feature_panel::FeatureSection::Discovery.title().to_owned(),
            rows,
        );
    }

    pub fn explore(&self) -> &feature_panel::FeaturePanel {
        &self.explore
    }

    pub fn feature_panel(&self) -> Option<&feature_panel::FeaturePanel> {
        self.feature_panel.as_ref()
    }

    pub(in crate::app) fn feed_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.feed_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.feed_viewer {
                    viewer.index = viewer.index.saturating_sub(1);
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.feed_viewer {
                    if viewer.index + 1 < viewer.items.len() {
                        viewer.index += 1;
                    }
                }
            }
            Key::Enter => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    self.feed_viewer = None;
                    self.dispatch(IoEvent::SelectFeed(feed));
                }
            }
            Key::Char('s') => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    if matches!(feed.kind, feed::FeedKind::Custom(_)) {
                        self.dispatch(IoEvent::ToggleSavedFeed(feed));
                    }
                }
            }
            Key::Char('!') => {
                let feed = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned();
                if let Some(feed::FeedDescriptor {
                    kind: feed::FeedKind::Custom(uri),
                    ..
                }) = feed
                {
                    self.feed_viewer = None;
                    self.open_report_prompt(feature_panel::ReportSubject::Feed(uri));
                }
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::FeedSearch);
                self.state.set_input(Input::default());
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn set_feed_catalog(&mut self, catalog: Vec<feed::FeedDescriptor>, open: bool) {
        self.feed_catalog = catalog.clone();
        if open {
            self.feed_viewer = Some(FeedViewer {
                items: catalog,
                index: 0,
            });
        }
    }

    pub fn set_feed_search_results(&mut self, results: Vec<feed::FeedDescriptor>) {
        self.feed_viewer = Some(FeedViewer {
            items: results,
            index: 0,
        });
    }

    pub fn current_feed_viewer(&self) -> Option<(Vec<feed::FeedDescriptor>, usize)> {
        let viewer = self.feed_viewer.as_ref()?;
        Some((viewer.items.clone(), viewer.index))
    }

    pub fn current_action_menu(&self) -> Option<(Vec<&'static str>, usize)> {
        let menu = self.action_menu.as_ref()?;
        Some((
            menu.items.iter().map(|(label, _)| *label).collect(),
            menu.index,
        ))
    }

    pub(in crate::app) fn action_menu_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.action_menu = None,
            Key::Up | Key::Char('k') => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = menu.index.saturating_sub(1);
                }
            }
            Key::Down | Key::Char('j') => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = (menu.index + 1).min(menu.items.len().saturating_sub(1));
                }
            }
            Key::Home => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = 0;
                }
            }
            Key::End => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = menu.items.len().saturating_sub(1);
                }
            }
            Key::Enter => {
                let selected = self
                    .action_menu
                    .take()
                    .and_then(|menu| menu.items.get(menu.index).map(|(_, key)| *key));
                if let Some(key) = selected {
                    return match self.state.get_tab() {
                        Tab::Home => self.timeline_action(key),
                        Tab::Notifications => self.notifications_action(key),
                        Tab::Messages => self.messages_action(key),
                        Tab::Search => self.search_action(key),
                    };
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }
}
