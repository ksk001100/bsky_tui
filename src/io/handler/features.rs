//! features effects.

use super::*;

impl IoAsyncHandler {
    pub(super) async fn handle_feature_event(&mut self, event: FeatureEvent) -> Result<()> {
        use crate::app::feature_panel::{FeaturePromptAction, FeatureSection, SettingKey};

        match event {
            FeatureEvent::Load(section) => self.load_feature_section(section).await,
            FeatureEvent::OpenList(uri) => {
                let did = self.did().await?;
                let rows = bsky::feature_services::list_detail(
                    self.agent().await?.as_ref(),
                    uri.clone(),
                    did,
                )
                .await?;
                self.emit(EffectMessage::FeatureRowsLoaded {
                    title: format!("List · {uri}"),
                    rows,
                    child: true,
                })
                .await;
                Ok(())
            }
            FeatureEvent::OpenStarterPack(uri) => {
                let rows = bsky::feature_services::starter_pack_detail(
                    self.agent().await?.as_ref(),
                    uri.clone(),
                )
                .await?;
                self.emit(EffectMessage::FeatureRowsLoaded {
                    title: format!("Starter Pack · {uri}"),
                    rows,
                    child: true,
                })
                .await;
                Ok(())
            }
            FeatureEvent::OpenConversation(convo_id) => self.open_conversation(convo_id).await,
            FeatureEvent::OpenLabeler(did) => {
                let rows = bsky::feature_services::labeler_detail(
                    self.agent().await?.as_ref(),
                    did.clone(),
                )
                .await?;
                self.emit(EffectMessage::FeatureRowsLoaded {
                    title: format!("Labeler · {}", did.as_str()),
                    rows,
                    child: true,
                })
                .await;
                Ok(())
            }
            FeatureEvent::Submit(action, value) => {
                let agent = self.agent().await?;
                match action {
                    FeaturePromptAction::CreateList => {
                        let fields = crate::app::feature_panel::split_fields(&value, 2)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::create_list(
                            agent.as_ref(),
                            fields[0].clone(),
                            fields[1].clone(),
                            fields.get(2).cloned().filter(|value| !value.is_empty()),
                        )
                        .await?;
                    }
                    FeaturePromptAction::EditList { uri, purpose } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 1)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::edit_list(
                            agent.as_ref(),
                            &uri,
                            purpose,
                            fields[0].clone(),
                            fields.get(1).cloned().filter(|value| !value.is_empty()),
                        )
                        .await?;
                    }
                    FeaturePromptAction::AddListMember { list_uri } => {
                        let actor = value.parse().map_err(eyre::Report::msg)?;
                        let profile = bsky::profile(agent.as_ref(), actor).await?;
                        bsky::feature_services::add_list_member(
                            agent.as_ref(),
                            list_uri,
                            profile.did.clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::CreateStarterPack => {
                        let fields = crate::app::feature_panel::split_fields(&value, 3)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::create_starter_pack(
                            agent.as_ref(),
                            fields[0].clone(),
                            Some(fields[1].clone()).filter(|value| !value.is_empty()),
                            fields[2].clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::EditStarterPack { uri } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 3)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::edit_starter_pack(
                            agent.as_ref(),
                            &uri,
                            fields[0].clone(),
                            Some(fields[1].clone()).filter(|value| !value.is_empty()),
                            fields[2].clone(),
                        )
                        .await?;
                    }
                    FeaturePromptAction::NewConversation => {
                        let actor = value.parse().map_err(eyre::Report::msg)?;
                        let profile = bsky::profile(agent.as_ref(), actor).await?;
                        let convo_id = bsky::feature_services::start_conversation(
                            agent.as_ref(),
                            profile.did.clone(),
                        )
                        .await?;
                        return self.open_conversation(convo_id).await;
                    }
                    FeaturePromptAction::SendMessage { convo_id } => {
                        bsky::feature_services::send_dm(agent.as_ref(), convo_id.clone(), value)
                            .await?;
                        return self.open_conversation(convo_id).await;
                    }
                    FeaturePromptAction::AddMutedWord => {
                        bsky::feature_services::add_muted_word(agent.as_ref(), value).await?;
                    }
                    FeaturePromptAction::AddLabeler => {
                        let did = atrium_api::types::string::Did::new(value)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::toggle_labeler(agent.as_ref(), did).await?;
                    }
                    FeaturePromptAction::SetLabelVisibility { labeler, label } => {
                        bsky::feature_services::set_label_visibility(
                            agent.as_ref(),
                            Some(labeler),
                            label,
                            value,
                        )
                        .await?;
                    }
                    FeaturePromptAction::Report { subject } => {
                        let fields = crate::app::feature_panel::split_fields(&value, 1)
                            .map_err(eyre::Report::msg)?;
                        bsky::feature_services::report(
                            agent.as_ref(),
                            subject,
                            &fields[0],
                            fields.get(1).cloned(),
                        )
                        .await?;
                        self.emit(EffectMessage::FeaturePanelClosed).await;
                        return Ok(());
                    }
                    FeaturePromptAction::AddAccount => {
                        let fields = crate::app::feature_panel::split_fields(&value, 2)
                            .map_err(eyre::Report::msg)?;
                        let mut config = crate::app::config::AppConfig::load()?;
                        if !config
                            .accounts
                            .iter()
                            .any(|account| account.identifier == fields[0])
                        {
                            config.accounts.push(crate::app::config::AccountConfig {
                                identifier: fields[0].clone(),
                                service_url: fields[1].clone(),
                            });
                        }
                        config.save()?;
                    }
                    FeaturePromptAction::EditSetting(setting) => {
                        if setting == SettingKey::IncomingDm {
                            bsky::feature_services::set_incoming_dm(agent.as_ref(), value).await?;
                        } else {
                            let mut config = crate::app::config::AppConfig::load()?;
                            match setting {
                                SettingKey::Images => {
                                    config.ui.show_images = value
                                        .parse()
                                        .map_err(|_| eyre!("images must be true or false"))?
                                }
                                SettingKey::DateFormat => config.ui.date_format = value,
                                SettingKey::Language => config.ui.language = value,
                                SettingKey::AccentColor => config.ui.accent_color = value,
                                SettingKey::Keybindings => {
                                    let fields = crate::app::feature_panel::split_fields(&value, 2)
                                        .map_err(eyre::Report::msg)?;
                                    config
                                        .ui
                                        .keybindings
                                        .insert(fields[0].clone(), fields[1].clone());
                                }
                                SettingKey::IncomingDm => {
                                    return Err(eyre!(
                                        "incoming DM is handled by the chat service"
                                    ));
                                }
                            }
                            config.save()?;
                            self.emit(EffectMessage::UiConfigLoaded(config.ui.clone()))
                                .await;
                        }
                    }
                }
                let section = self
                    .context
                    .as_ref()
                    .and_then(|context| context.feature_panel_section)
                    .unwrap_or(FeatureSection::Lists);
                self.load_feature_section(section).await
            }
            FeatureEvent::DeleteRecord(uri) => {
                bsky::feature_services::delete_record(self.agent().await?.as_ref(), &uri).await?;
                let section = self
                    .context
                    .as_ref()
                    .and_then(|context| context.feature_panel_section)
                    .unwrap_or(FeatureSection::Lists);
                self.load_feature_section(section).await
            }
            FeatureEvent::ToggleModerationList { uri, muted } => {
                bsky::feature_services::toggle_moderation_list(
                    self.agent().await?.as_ref(),
                    uri,
                    muted,
                )
                .await?;
                self.load_feature_section(FeatureSection::Lists).await
            }
            FeatureEvent::ToggleConversationMute { convo_id, muted } => {
                bsky::feature_services::toggle_conversation_mute(
                    self.agent().await?.as_ref(),
                    convo_id,
                    muted,
                )
                .await?;
                self.load_feature_section(FeatureSection::DirectMessages)
                    .await
            }
            FeatureEvent::RemoveMutedWord(word) => {
                bsky::feature_services::remove_muted_word(self.agent().await?.as_ref(), &word)
                    .await?;
                self.load_feature_section(FeatureSection::Moderation).await
            }
            FeatureEvent::ToggleLabeler(did) => {
                bsky::feature_services::toggle_labeler(self.agent().await?.as_ref(), did).await?;
                self.load_feature_section(FeatureSection::Moderation).await
            }
            FeatureEvent::UseListFeed { uri, name } => {
                self.emit(EffectMessage::FeedActivated(
                    crate::app::feed::FeedDescriptor::list(uri, name),
                ))
                .await;
                self.load_timeline(TimelineEvent::Load).await
            }
            FeatureEvent::SaveList(uri) => {
                let mut config = crate::app::config::AppConfig::load()?;
                if config.saved_lists.iter().any(|saved| saved == &uri) {
                    config.saved_lists.retain(|saved| saved != &uri);
                } else {
                    config.saved_lists.push(uri);
                }
                config.save()?;
                self.load_feature_section(FeatureSection::Lists).await
            }
            FeatureEvent::JoinStarterPack(actors) => {
                let agent = self.agent().await?;
                for actor in actors {
                    let profile = bsky::profile(agent.as_ref(), actor).await?;
                    if profile
                        .viewer
                        .as_ref()
                        .and_then(|viewer| viewer.following.as_ref())
                        .is_none()
                    {
                        bsky::toggle_follow(agent.as_ref(), &profile).await?;
                    }
                }
                Ok(())
            }
            FeatureEvent::SwitchAccount(identifier) => {
                let mut config = crate::app::config::AppConfig::load()?;
                config.active_account = Some(identifier);
                config.save()?;
                self.emit(EffectMessage::FeaturePanelClosed).await;
                self.initialize().await
            }
            FeatureEvent::ToggleThreadMute { root, muted } => {
                bsky::feature_services::toggle_thread_mute(
                    self.agent().await?.as_ref(),
                    root.clone(),
                    muted,
                )
                .await?;
                let mut config = crate::app::config::AppConfig::load()?;
                if muted {
                    config.muted_threads.retain(|uri| uri != &root);
                } else if !config.muted_threads.contains(&root) {
                    config.muted_threads.push(root);
                }
                config.save()?;
                if self
                    .context
                    .as_ref()
                    .is_some_and(|context| context.feature_panel_open)
                {
                    self.load_feature_section(crate::app::feature_panel::FeatureSection::Moderation)
                        .await
                } else {
                    Ok(())
                }
            }
            FeatureEvent::ToggleHiddenReply { root, reply } => {
                let uri = root.uri.clone();
                bsky::feature_services::toggle_hidden_reply(
                    self.agent().await?.as_ref(),
                    &root,
                    reply,
                )
                .await?;
                self.load_thread(uri).await
            }
            FeatureEvent::DetachQuote { post, quote } => {
                bsky::feature_services::detach_quote(
                    self.agent().await?.as_ref(),
                    post,
                    quote.clone(),
                )
                .await?;
                self.load_thread(quote).await
            }
        }
    }

    pub(super) async fn load_feature_section(
        &mut self,
        section: crate::app::feature_panel::FeatureSection,
    ) -> Result<()> {
        use crate::app::feature_panel::{FeatureRow, FeatureSection, FeatureTarget, SettingKey};
        let did = self.did().await?;
        let rows = match section {
            FeatureSection::Lists => {
                let agent = self.agent().await?;
                let mut rows =
                    bsky::feature_services::own_lists(agent.as_ref(), did.clone()).await?;
                let config = crate::app::config::AppConfig::load()?;
                for uri in config.saved_lists {
                    if !rows.iter().any(|row| matches!(&row.target, FeatureTarget::List { uri: existing, .. } if existing == &uri)) {
                        if let Ok(row) = bsky::feature_services::list_overview(agent.as_ref(), uri, &did).await {
                            rows.push(row);
                        }
                    }
                }
                rows
            }
            FeatureSection::StarterPacks => {
                bsky::feature_services::starter_packs(self.agent().await?.as_ref(), did).await?
            }
            FeatureSection::Discovery => {
                bsky::feature_services::discovery(self.agent().await?.as_ref(), did).await?
            }
            FeatureSection::DirectMessages => {
                bsky::feature_services::conversations(self.agent().await?.as_ref(), &did).await?
            }
            FeatureSection::Moderation => {
                let mut rows = bsky::feature_services::moderation_preferences_rows(
                    self.agent().await?.as_ref(),
                )
                .await?;
                let config = crate::app::config::AppConfig::load()?;
                rows.extend(config.muted_threads.into_iter().map(|uri| FeatureRow {
                    title: "Muted thread".into(),
                    detail: uri.clone(),
                    target: FeatureTarget::MutedThread(uri),
                    unread: false,
                }));
                rows
            }
            FeatureSection::Settings => {
                let config = crate::app::config::AppConfig::load()?;
                let mut rows = config
                    .all_accounts()
                    .into_iter()
                    .map(|account| FeatureRow {
                        title: format!(
                            "{}{}",
                            if config.active_account().identifier == account.identifier {
                                "● "
                            } else {
                                "  "
                            },
                            account.identifier
                        ),
                        detail: account.service_url,
                        target: FeatureTarget::Account(account.identifier),
                        unread: false,
                    })
                    .collect::<Vec<_>>();
                rows.extend([
                    setting_row(
                        "Images",
                        config.ui.show_images.to_string(),
                        SettingKey::Images,
                    ),
                    setting_row("Date format", config.ui.date_format, SettingKey::DateFormat),
                    setting_row("Language", config.ui.language, SettingKey::Language),
                    setting_row(
                        "Accent color",
                        config.ui.accent_color,
                        SettingKey::AccentColor,
                    ),
                    setting_row("Keybinding", "action | key".into(), SettingKey::Keybindings),
                    setting_row(
                        "Incoming DMs",
                        "all / following / none".into(),
                        SettingKey::IncomingDm,
                    ),
                ]);
                rows
            }
        };
        if section == FeatureSection::DirectMessages {
            self.emit(EffectMessage::MessagesReplaced {
                title: section.title().to_owned(),
                rows,
            })
            .await;
        } else if section == FeatureSection::Discovery && self.state().get_tab() == Tab::Search {
            self.emit(EffectMessage::ExploreReplaced(rows)).await;
        } else {
            self.emit(EffectMessage::FeatureRowsLoaded {
                title: section.title().to_owned(),
                rows,
                child: false,
            })
            .await;
        }
        Ok(())
    }

    pub(super) async fn open_conversation(&mut self, convo_id: String) -> Result<()> {
        let rows =
            bsky::feature_services::conversation(self.agent().await?.as_ref(), convo_id.clone())
                .await?;
        self.emit(EffectMessage::ConversationLoaded {
            title: format!("Conversation · {convo_id}"),
            rows,
        })
        .await;
        Ok(())
    }
}
