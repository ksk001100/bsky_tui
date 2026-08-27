use std::sync::Arc;

use seahorse::{App as SeahorseApp, Command, Context};

use bsky_tui::{
    app::{config::AppConfig, App},
    io::{handler::IoAsyncHandler, IoEvent},
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
        .run(std::env::args().collect());
}

#[tokio::main]
async fn action(_c: &Context) {
    if !AppConfig::config_exists() {
        let path = AppConfig::config_path();
        println!("Config file not found: {}", path.display());
        println!("Run `bsky_tui generate` to generate a config file");
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

    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<IoEvent>(100);

    let app = Arc::new(tokio::sync::Mutex::new(App::new(sync_io_tx.clone())));
    let app_ui = Arc::clone(&app);

    tokio::spawn(async move {
        let mut handler = IoAsyncHandler::new(app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_io_event(io_event).await;
        }
    });

    if let Err(error) = start_ui(&app_ui, config.skip_splash, get_splash(config.splash_path)).await
    {
        eprintln!("Application error: {error:#}");
    }
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
