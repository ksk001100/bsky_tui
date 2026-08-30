//! Effects for feature-panel domains.

use super::*;

mod chat;
mod discovery;
mod lists;
mod moderation;
mod settings;
mod starter_packs;

impl IoAsyncHandler {
    pub(super) async fn handle_feature_event(&mut self, event: FeatureEvent) -> Result<()> {
        use crate::app::feature_panel::{FeaturePromptAction, FeatureSection};

        match event {
            FeatureEvent::Load(section) => self.load_feature_section(section).await,
            FeatureEvent::OpenList(uri) => self.open_list(uri).await,
            FeatureEvent::OpenStarterPack(uri) => self.open_starter_pack(uri).await,
            FeatureEvent::OpenConversation(convo_id) => self.open_conversation(convo_id).await,
            FeatureEvent::OpenLabeler(did) => self.open_labeler(did).await,
            FeatureEvent::Submit(action, value) => {
                // Preserve the previous invariant that every submission requires a live session.
                self.agent().await?;
                match action {
                    FeaturePromptAction::CreateList => self.create_list(value).await?,
                    FeaturePromptAction::EditList { uri, purpose } => {
                        self.edit_list(uri, purpose, value).await?
                    }
                    FeaturePromptAction::AddListMember { list_uri } => {
                        self.add_list_member(list_uri, value).await?
                    }
                    FeaturePromptAction::CreateStarterPack => {
                        self.create_starter_pack(value).await?
                    }
                    FeaturePromptAction::EditStarterPack { uri } => {
                        self.edit_starter_pack(uri, value).await?
                    }
                    FeaturePromptAction::NewConversation => {
                        return self.start_conversation(value).await;
                    }
                    FeaturePromptAction::SendMessage { convo_id } => {
                        return self.send_message(convo_id, value).await;
                    }
                    FeaturePromptAction::AddMutedWord => self.add_muted_word(value).await?,
                    FeaturePromptAction::AddLabeler => self.add_labeler(value).await?,
                    FeaturePromptAction::SetLabelVisibility { labeler, label } => {
                        self.set_label_visibility(labeler, label, value).await?
                    }
                    FeaturePromptAction::Report { subject } => {
                        self.report(subject, value).await?;
                        self.emit(EffectMessage::FeaturePanelClosed).await;
                        return Ok(());
                    }
                    FeaturePromptAction::AddAccount => self.add_account(value)?,
                    FeaturePromptAction::EditSetting(setting) => {
                        self.edit_setting(setting, value).await?
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
                self.toggle_moderation_list(uri, muted).await
            }
            FeatureEvent::ToggleConversationMute { convo_id, muted } => {
                self.toggle_conversation_mute(convo_id, muted).await
            }
            FeatureEvent::RemoveMutedWord(word) => self.remove_muted_word(word).await,
            FeatureEvent::ToggleLabeler(did) => self.toggle_labeler(did).await,
            FeatureEvent::UseListFeed { uri, name } => self.use_list_feed(uri, name).await,
            FeatureEvent::SaveList(uri) => self.save_list(uri).await,
            FeatureEvent::JoinStarterPack(actors) => self.join_starter_pack(actors).await,
            FeatureEvent::SwitchAccount(identifier) => self.switch_account(identifier).await,
            FeatureEvent::ToggleThreadMute { root, muted } => {
                self.toggle_thread_mute(root, muted).await
            }
            FeatureEvent::ToggleHiddenReply { root, reply } => {
                self.toggle_hidden_reply(root, reply).await
            }
            FeatureEvent::DetachQuote { post, quote } => self.detach_quote(post, quote).await,
        }
    }

    pub(super) async fn load_feature_section(
        &mut self,
        section: crate::app::feature_panel::FeatureSection,
    ) -> Result<()> {
        use crate::app::feature_panel::FeatureSection;

        let rows = match section {
            FeatureSection::Lists => self.load_lists().await?,
            FeatureSection::StarterPacks => self.load_starter_packs().await?,
            FeatureSection::Discovery => self.load_discovery().await?,
            FeatureSection::DirectMessages => self.load_conversations().await?,
            FeatureSection::Moderation => self.load_moderation().await?,
            FeatureSection::Settings => self.load_settings().await?,
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
}
