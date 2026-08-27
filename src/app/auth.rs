use eyre::{bail, eyre, Result, WrapErr};
use zeroize::Zeroizing;

const KEYRING_SERVICE: &str = "dev.ksk001100.bsky_tui";
const PASSWORD_ENV: &str = "BSKY_TUI_APP_PASSWORD";

/// Authentication material kept outside serializable application settings and UI state.
pub struct AuthCredentials {
    app_password: Zeroizing<String>,
}

impl AuthCredentials {
    pub fn load(identifier: &str) -> Result<Self> {
        let entry = keyring_entry(identifier)?;
        let keyring_result = match entry.get_password() {
            Ok(app_password) if !app_password.trim().is_empty() => {
                return Ok(Self {
                    app_password: Zeroizing::new(app_password),
                });
            }
            result => result,
        };

        let app_password = std::env::var(PASSWORD_ENV).unwrap_or_default();
        if !app_password.trim().is_empty() {
            return Ok(Self {
                app_password: Zeroizing::new(app_password),
            });
        }

        match keyring_result {
            Err(keyring::Error::NoEntry) | Ok(_) => bail!(
                "no App Password is saved; run `bsky_tui credentials set` or set {PASSWORD_ENV}"
            ),
            Err(error) => Err(eyre!(error)).wrap_err(format!(
                "could not read the OS keyring; run `bsky_tui credentials set` or set {PASSWORD_ENV}"
            )),
        }
    }

    pub fn save(identifier: &str, app_password: &str) -> Result<()> {
        validate_identifier(identifier)?;
        if app_password.trim().is_empty() {
            bail!("App Password cannot be empty");
        }
        keyring_entry(identifier)?
            .set_password(app_password)
            .map_err(eyre::Report::msg)
            .wrap_err("could not save the App Password to the OS keyring")
    }

    pub fn delete(identifier: &str) -> Result<bool> {
        match keyring_entry(identifier)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(error) => Err(eyre!(error)).wrap_err("could not delete the OS keyring entry"),
        }
    }

    pub fn app_password(&self) -> &str {
        self.app_password.as_str()
    }
}

fn keyring_entry(identifier: &str) -> Result<keyring::Entry> {
    validate_identifier(identifier)?;
    keyring::Entry::new(KEYRING_SERVICE, identifier)
        .map_err(eyre::Report::msg)
        .wrap_err("could not access the OS keyring")
}

fn validate_identifier(identifier: &str) -> Result<()> {
    if identifier.trim().is_empty() {
        bail!("identifier is required before accessing credentials");
    }
    Ok(())
}

impl std::fmt::Debug for AuthCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthCredentials")
            .field("app_password", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_password() {
        let credentials = AuthCredentials {
            app_password: Zeroizing::new("secret-password".to_owned()),
        };

        let output = format!("{credentials:?}");
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("secret-password"));
    }

    #[test]
    fn empty_identifier_is_rejected_before_keyring_access() {
        assert!(keyring_entry(" ").is_err());
    }
}
