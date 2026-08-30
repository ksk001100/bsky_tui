//! timeline update handlers.

use super::super::*;

impl App {
    pub(in crate::app) fn timeline_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::F5 => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload));
                AppReturn::Continue
            }
            Key::Char('c') => {
                if self.feed_catalog.is_empty() {
                    self.dispatch(IoEvent::LoadFeedCatalog);
                } else {
                    self.feed_viewer = Some(FeedViewer {
                        items: self.feed_catalog.clone(),
                        index: 0,
                    });
                }
                AppReturn::Continue
            }
            Key::Char('n') => {
                self.state.set_mode(state::Mode::Post);
                AppReturn::Continue
            }
            Key::Char('r') => {
                self.state.set_mode(state::Mode::Reply);
                AppReturn::Continue
            }
            Key::Char('X') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.start_quote_composer(&feed.post);
                }
                AppReturn::Continue
            }
            Key::Ctrl('r') => {
                self.dispatch(IoEvent::Repost);
                AppReturn::Continue
            }
            Key::Ctrl('l') => {
                self.dispatch(IoEvent::Like);
                AppReturn::Continue
            }
            Key::Char('b') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::ToggleBookmark(Box::new(feed.post.data.clone())));
                }
                AppReturn::Continue
            }
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::Search);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('u') => {
                self.state.set_mode(state::Mode::UserSearch);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_tl_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_tl_scroll_up();
                AppReturn::Continue
            }
            Key::PageDown | Key::Ctrl('d') => {
                self.state.move_tl_half_down();
                AppReturn::Continue
            }
            Key::PageUp | Key::Ctrl('u') => {
                self.state.move_tl_half_up();
                AppReturn::Continue
            }
            Key::Home => {
                self.state.move_tl_scroll_top();
                AppReturn::Continue
            }
            Key::End => {
                self.state.move_tl_scroll_bottom();
                AppReturn::Continue
            }
            Key::Char('y') | Key::Char('Y') | Key::Alt('y') => {
                self.copy_selected_post(key);
                AppReturn::Continue
            }
            Key::Char('o') => {
                if let Some(feed) = self.state.get_current_feed() {
                    if let Some(id) = feed.post.uri.split('/').next_back() {
                        let handle = &feed.post.author.handle;
                        let url =
                            format!("https://bsky.app/profile/{}/post/{}", handle.as_str(), id);
                        self.open_url(url, "Could not open timeline post");
                    }
                }
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::LoadThread(feed.post.uri.clone()));
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.dispatch(IoEvent::LoadProfile(feed.post.author.did.clone().into()));
                }
                AppReturn::Continue
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.request_post_moderation(&feed.post, key);
                }
                AppReturn::Continue
            }
            Key::Char('D') => {
                if let Some(feed) = self.state.get_current_feed() {
                    self.request_delete(&feed.post);
                }
                AppReturn::Continue
            }
            Key::Char('e') => {
                self.open_selected_embed(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Char('f') => {
                self.open_facet_viewer(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(feed) = self.state.get_current_feed() {
                    let kind = interaction_kind(key);
                    self.dispatch(IoEvent::LoadInteractions(
                        kind,
                        feed.post.uri.clone(),
                        feed.post.cid.clone(),
                    ));
                }
                AppReturn::Continue
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(
                    self.state
                        .get_current_feed()
                        .map(|feed| feed.post.data.clone()),
                );
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                match self.state.get_tab() {
                    Tab::Home => {
                        self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload));
                    }
                    Tab::Notifications => {
                        self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load));
                    }
                    Tab::Messages => {
                        self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                            feature_panel::FeatureSection::DirectMessages,
                        )));
                    }
                    Tab::Search => {}
                }
                AppReturn::Continue
            }
            Key::Char('h') | Key::Left | Key::Char('[') => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Prev));
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') => {
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Next));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn messages_action(&mut self, key: Key) -> AppReturn {
        if self.messages.prompt.is_some() {
            return self.message_prompt_action(key);
        }

        let selected = self.messages.selected_row().cloned();
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::F5 => {
                if let Some(convo_id) = self
                    .messages
                    .title
                    .strip_prefix("Conversation · ")
                    .map(str::to_owned)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenConversation(convo_id)));
                } else {
                    self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                        feature_panel::FeatureSection::DirectMessages,
                    )));
                }
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.messages.previous();
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.messages.next();
                AppReturn::Continue
            }
            Key::Enter | Key::Char('o') => {
                if let Some(feature_panel::FeatureTarget::Conversation { id, .. }) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::OpenConversation(id)));
                }
                AppReturn::Continue
            }
            Key::Char('n') => {
                self.open_message_prompt(
                    "New conversation".into(),
                    "Handle or DID".into(),
                    feature_panel::FeaturePromptAction::NewConversation,
                );
                AppReturn::Continue
            }
            Key::Char('w') => {
                let convo_id = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Conversation { id, .. }) => Some(id),
                    Some(feature_panel::FeatureTarget::Message { convo_id, .. }) => Some(convo_id),
                    _ => None,
                }
                .or_else(|| {
                    self.messages
                        .title
                        .strip_prefix("Conversation · ")
                        .map(str::to_owned)
                });
                if let Some(convo_id) = convo_id {
                    self.open_message_prompt(
                        "Send message".into(),
                        "Text message (1000 characters maximum)".into(),
                        feature_panel::FeaturePromptAction::SendMessage { convo_id },
                    );
                }
                AppReturn::Continue
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
                AppReturn::Continue
            }
            Key::Char('r') => {
                let subject = match selected.map(|row| row.target) {
                    Some(feature_panel::FeatureTarget::Message {
                        convo_id,
                        id,
                        sender,
                    }) => Some(feature_panel::ReportSubject::Conversation {
                        convo_id,
                        message_id: Some(format!("{id}@{}", sender.as_str())),
                        sender,
                    }),
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
                    self.open_message_prompt(
                        "Report".into(),
                        "spam|rude|sexual|violation|misleading|other | details".into(),
                        feature_panel::FeaturePromptAction::Report { subject },
                    );
                }
                AppReturn::Continue
            }
            Key::Char('b') => {
                let did = match selected.map(|row| row.target) {
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
                AppReturn::Continue
            }
            Key::Esc => {
                if let Some(parent) = self.messages.parent.take().map(|parent| *parent) {
                    self.messages = parent;
                }
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::Discovery,
                )));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn message_prompt_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.messages.prompt = None,
            Key::Enter => {
                if let Some(prompt) = self.messages.prompt.take() {
                    let value = prompt.input.value().trim().to_owned();
                    self.dispatch(IoEvent::Feature(FeatureEvent::Submit(prompt.action, value)));
                }
            }
            Key::Left | Key::Ctrl('b') => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::GoToPrevChar);
                }
            }
            Key::Right | Key::Ctrl('f') => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::GoToNextChar);
                }
            }
            Key::Ctrl('a') => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::GoToStart);
                }
            }
            Key::Ctrl('e') => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::GoToEnd);
                }
            }
            Key::Backspace | Key::Ctrl('h') => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::DeletePrevChar);
                }
            }
            Key::Char(character) => {
                if let Some(prompt) = self.messages.prompt.as_mut() {
                    prompt.input.handle(InputRequest::InsertChar(character));
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn open_message_prompt(
        &mut self,
        label: String,
        help: String,
        action: feature_panel::FeaturePromptAction,
    ) {
        self.messages.prompt = Some(feature_panel::FeaturePrompt {
            label,
            help,
            action,
            input: Input::default(),
        });
    }
}
