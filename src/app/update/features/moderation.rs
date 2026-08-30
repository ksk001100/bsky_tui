use super::*;

impl App {
    pub(super) fn moderation_action(
        &mut self,
        key: Key,
        selected: Option<feature_panel::FeatureRow>,
    ) {
        match key {
            Key::Char('n') => self.open_feature_prompt(
                "Mute word".into(),
                "Word or phrase".into(),
                feature_panel::FeaturePromptAction::AddMutedWord,
                String::new(),
            ),
            Key::Char('e') => {
                if let Some(feature_panel::FeatureTarget::LabelSetting { labeler, label }) =
                    selected.map(|row| row.target)
                {
                    self.open_feature_prompt(
                        format!("Label visibility · {label}"),
                        "ignore / warn / hide".into(),
                        feature_panel::FeaturePromptAction::SetLabelVisibility { labeler, label },
                        "warn".into(),
                    );
                }
            }
            Key::Char('x') | Key::Delete => match selected.map(|row| row.target) {
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
            Key::Char('l') => {
                if let Some(feature_panel::FeatureTarget::Labeler(did)) =
                    selected.map(|row| row.target)
                {
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleLabeler(did)));
                }
            }
            Key::Char('L') => self.open_feature_prompt(
                "Subscribe to labeler".into(),
                "Labeler DID".into(),
                feature_panel::FeaturePromptAction::AddLabeler,
                String::new(),
            ),
            _ => {}
        }
    }

    pub(super) fn cross_domain_moderation_action(
        &mut self,
        key: Key,
        selected: Option<feature_panel::FeatureRow>,
    ) {
        match key {
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
    }
}
