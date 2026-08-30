use super::*;

impl IoAsyncHandler {
    pub(super) async fn load_settings(
        &mut self,
    ) -> Result<Vec<crate::app::feature_panel::FeatureRow>> {
        use crate::app::feature_panel::{FeatureRow, FeatureTarget, SettingKey};

        // Loading every feature section previously verified the initialized account first.
        self.did().await?;
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
        Ok(rows)
    }

    pub(super) fn add_account(&mut self, value: String) -> Result<()> {
        let fields =
            crate::app::feature_panel::split_fields(&value, 2).map_err(eyre::Report::msg)?;
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
        config.save()
    }

    pub(super) async fn edit_setting(
        &mut self,
        setting: crate::app::feature_panel::SettingKey,
        value: String,
    ) -> Result<()> {
        use crate::app::feature_panel::SettingKey;

        if setting == SettingKey::IncomingDm {
            return bsky::feature_services::set_incoming_dm(self.agent().await?.as_ref(), value)
                .await;
        }
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
                return Err(eyre!("incoming DM is handled by the chat service"));
            }
        }
        config.save()?;
        self.emit(EffectMessage::UiConfigLoaded(config.ui.clone()))
            .await;
        Ok(())
    }

    pub(super) async fn switch_account(&mut self, identifier: String) -> Result<()> {
        let mut config = crate::app::config::AppConfig::load()?;
        config.active_account = Some(identifier);
        config.save()?;
        self.emit(EffectMessage::FeaturePanelClosed).await;
        self.initialize().await
    }
}
