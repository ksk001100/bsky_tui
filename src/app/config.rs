use std::io::Write;

use config::Config;
use eyre::Result;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub email: String,
    pub password: String,
    pub skip_splash: bool,
}

impl AppConfig {
    pub fn new(email: String, password: String, skip_splash: bool) -> Self {
        Self {
            email,
            password,
            skip_splash,
        }
    }

    pub fn load() -> Result<Self> {
        let config = Self::read_config()?.try_deserialize()?;
        Ok(config)
    }

    pub fn check_required_fields(&self) -> Result<()> {
        if self.email.is_empty() {
            eyre::bail!("email is required");
        }
        if self.password.is_empty() {
            eyre::bail!("password is required");
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

        let content = toml::to_string(&Self::default())?;
        let mut file = std::fs::File::create(&path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    fn read_config() -> Result<Config> {
        let path = Self::config_path();
        let config = Config::builder()
            .set_default("skip_splash", false)?
            .add_source(
                config::File::from(path)
                    .required(true)
                    .format(config::FileFormat::Toml),
            )
            .build()?;
        Ok(config)
    }
}
