use std::sync::Arc;

use seahorse::{App as SeahorseApp, Command, Context};

use bsky_tui::{
    app::{auth::AuthCredentials, command::Command as AppCommand, config::AppConfig, App},
    io::handler::{EffectEnvelope, IoAsyncHandler},
    start_ui,
    utils::get_splash,
};

fn main() {
    SeahorseApp::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage(format!("{} [args]", env!("CARGO_PKG_NAME")))
        .action(action)
        .command(config_command())
        .command(credentials_command())
        .run(std::env::args().collect());
}

#[tokio::main]
async fn action(_c: &Context) {
    if !AppConfig::config_exists() {
        let path = AppConfig::config_path();
        println!("Config file not found: {}", path.display());
        println!("Run `bsky_tui config` to generate a config file");
        return;
    }

    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load config: {error:#}");
            return;
        }
    };
    if let Err(e) = config.check_required_fields() {
        println!("Config file error: {}", e);
        return;
    }

    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel::<AppCommand>(100);
    let (effect_tx, effect_rx) = tokio::sync::mpsc::channel::<EffectEnvelope>(100);

    let app = Arc::new(tokio::sync::Mutex::new(App::new()));
    let app_ui = Arc::clone(&app);

    tokio::spawn(async move {
        let mut handler = IoAsyncHandler::new(effect_tx);
        while let Some(command) = command_rx.recv().await {
            match command {
                AppCommand::Io { event, context } => handler.handle_io_event(event, *context).await,
                AppCommand::LoadImages(_)
                | AppCommand::PollImages
                | AppCommand::OpenUrl { .. }
                | AppCommand::CopyToClipboard { .. } => {}
            }
        }
    });

    if let Err(error) = start_ui(
        &app_ui,
        command_tx,
        effect_rx,
        config.skip_splash,
        get_splash(config.splash_path),
    )
    .await
    {
        eprintln!("Application error: {error:#}");
    }
}

fn credentials_command() -> Command {
    Command::new("credentials")
        .description("Manage the App Password in the OS keyring")
        .command(
            Command::new("set")
                .description("Save or replace the App Password")
                .action(|_| {
                    if let Err(error) = set_credentials() {
                        eprintln!("Failed to save credentials: {error:#}");
                    }
                }),
        )
        .command(
            Command::new("delete")
                .description("Delete the saved App Password")
                .action(|_| {
                    if let Err(error) = delete_credentials() {
                        eprintln!("Failed to delete credentials: {error:#}");
                    }
                }),
        )
}

fn set_credentials() -> eyre::Result<()> {
    let config = load_checked_config()?;
    let account = config.active_account();
    let password = zeroize::Zeroizing::new(rpassword::prompt_password("Bluesky App Password: ")?);
    let confirmation =
        zeroize::Zeroizing::new(rpassword::prompt_password("Confirm App Password: ")?);
    if password.as_str() != confirmation.as_str() {
        eyre::bail!("App Passwords do not match");
    }
    AuthCredentials::save(&account.identifier, password.as_str())?;
    println!(
        "App Password saved in the OS keyring for {}.",
        account.identifier
    );
    Ok(())
}

fn delete_credentials() -> eyre::Result<()> {
    let config = load_checked_config()?;
    let account = config.active_account();
    if AuthCredentials::delete(&account.identifier)? {
        println!("Saved App Password deleted for {}.", account.identifier);
    } else {
        println!(
            "No saved App Password was found for {}.",
            account.identifier
        );
    }
    Ok(())
}

fn load_checked_config() -> eyre::Result<AppConfig> {
    if !AppConfig::config_exists() {
        eyre::bail!("config file not found; run `bsky_tui config` first");
    }
    let config = AppConfig::load()?;
    config.check_required_fields()?;
    Ok(config)
}

fn config_command() -> Command {
    Command::new("config")
        .description("Generate config file")
        .alias("c")
        .action(|_| {
            if AppConfig::config_exists() {
                println!(
                    "Config file already exists: {}",
                    AppConfig::config_path().display()
                );
                return;
            }
            match AppConfig::generate_config_file() {
                Ok(()) => println!(
                    "Config file generated at: {}",
                    AppConfig::config_path().display()
                ),
                Err(error) => eprintln!("Failed to generate config: {error:#}"),
            }
        })
}
