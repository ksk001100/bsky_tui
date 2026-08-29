use std::{collections::HashMap, io::Write};

use config::Config;
use eyre::Result;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccountConfig {
    pub identifier: String,
    #[serde(default = "default_service_url")]
    pub service_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub show_images: bool,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    #[serde(default)]
    pub keybindings: HashMap<String, String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_images: true,
            date_format: default_date_format(),
            language: default_language(),
            accent_color: default_accent_color(),
            keybindings: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(alias = "email")]
    pub identifier: String,
    #[serde(default = "default_service_url")]
    pub service_url: String,
    pub skip_splash: bool,
    pub splash_path: Option<String>,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub active_account: Option<String>,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub saved_lists: Vec<String>,
    #[serde(default)]
    pub muted_threads: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            identifier: String::new(),
            service_url: default_service_url(),
            skip_splash: false,
            splash_path: None,
            accounts: Vec::new(),
            active_account: None,
            ui: UiConfig::default(),
            saved_lists: Vec::new(),
            muted_threads: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn new(identifier: String, skip_splash: bool, splash_path: Option<String>) -> Self {
        Self {
            identifier,
            service_url: default_service_url(),
            skip_splash,
            splash_path,
            accounts: Vec::new(),
            active_account: None,
            ui: UiConfig::default(),
            saved_lists: Vec::new(),
            muted_threads: Vec::new(),
        }
    }

    pub fn load() -> Result<Self> {
        let config = Self::read_config()?.try_deserialize()?;
        Ok(config)
    }

    pub fn check_required_fields(&self) -> Result<()> {
        if self.active_account().identifier.trim().is_empty() {
            eyre::bail!("identifier is required");
        }
        if self.active_account().service_url.trim().is_empty() {
            eyre::bail!("service_url is required");
        }
        Ok(())
    }

    pub fn active_account(&self) -> AccountConfig {
        self.active_account
            .as_deref()
            .and_then(|active| {
                self.accounts
                    .iter()
                    .find(|account| account.identifier == active)
            })
            .cloned()
            .unwrap_or_else(|| AccountConfig {
                identifier: self.identifier.clone(),
                service_url: self.service_url.clone(),
            })
    }

    pub fn all_accounts(&self) -> Vec<AccountConfig> {
        let mut accounts = self.accounts.clone();
        if !self.identifier.trim().is_empty()
            && !accounts
                .iter()
                .any(|account| account.identifier == self.identifier)
        {
            accounts.insert(
                0,
                AccountConfig {
                    identifier: self.identifier.clone(),
                    service_url: self.service_url.clone(),
                },
            );
        }
        accounts
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let prefix = path
            .parent()
            .ok_or_else(|| eyre::eyre!("invalid config path"))?;
        std::fs::create_dir_all(prefix)?;
        let content = toml::to_string_pretty(self)?;
        let temporary = path.with_extension("toml.tmp");
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?
        };
        #[cfg(not(unix))]
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn config_exists() -> bool {
        Self::config_path().exists()
    }

    pub fn config_path() -> std::path::PathBuf {
        let config_dir = dirs::config_dir().unwrap();
        config_dir.join("bsky_tui/config.toml")
    }

    pub fn generate_config_file() -> Result<()> {
        let path = Self::config_path();
        let prefix = path.parent().unwrap();
        std::fs::create_dir_all(prefix)?;

        let content = format!(
            "# Store the App Password with `bsky_tui credentials set`.\n{}",
            toml::to_string(&Self::default())?
        );
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?
        };
        #[cfg(not(unix))]
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    fn read_config() -> Result<Config> {
        let path = Self::config_path();
        let config = Config::builder()
            .set_default("skip_splash", false)?
            .set_default("splash_path", None::<String>)?
            .set_default("service_url", default_service_url())?
            .add_source(
                config::File::from(path)
                    .required(true)
                    .format(config::FileFormat::Toml),
            )
            .build()?;
        Ok(config)
    }
}

fn default_service_url() -> String {
    "https://bsky.social".to_string()
}

fn default_true() -> bool {
    true
}

fn default_date_format() -> String {
    "%Y-%m-%d %H:%M".to_owned()
}

fn default_language() -> String {
    "auto".to_owned()
}

fn default_accent_color() -> String {
    "blue".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_account_remains_the_default() {
        let config = AppConfig::new("alice.test".into(), false, None);
        assert_eq!(config.active_account().identifier, "alice.test");
        assert_eq!(config.all_accounts().len(), 1);
    }

    #[test]
    fn named_active_account_takes_precedence() {
        let config = AppConfig {
            accounts: vec![AccountConfig {
                identifier: "bob.test".into(),
                service_url: "https://pds.example".into(),
            }],
            active_account: Some("bob.test".into()),
            ..Default::default()
        };
        assert_eq!(config.active_account().service_url, "https://pds.example");
    }
}
