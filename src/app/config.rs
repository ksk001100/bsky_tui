use std::io::Write;

use config::Config;
use eyre::Result;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(alias = "email")]
    pub identifier: String,
    #[serde(default = "default_service_url")]
    pub service_url: String,
    pub skip_splash: bool,
    pub splash_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            identifier: String::new(),
            service_url: default_service_url(),
            skip_splash: false,
            splash_path: None,
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
        }
    }

    pub fn load() -> Result<Self> {
        let config = Self::read_config()?.try_deserialize()?;
        Ok(config)
    }

    pub fn check_required_fields(&self) -> Result<()> {
        if self.identifier.trim().is_empty() {
            eyre::bail!("identifier is required");
        }
        if self.service_url.trim().is_empty() {
            eyre::bail!("service_url is required");
        }
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
