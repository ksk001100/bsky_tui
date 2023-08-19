use std::sync::Arc;

use seahorse::{App as SeahorseApp, Command, Context};

use bsky_tui::utils::get_splash;
use bsky_tui::{
    app::{config::AppConfig, App},
    io::{handler::IoAsyncHandler, IoEvent},
    start_ui,
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
        println!("Config file not found: {}", path.to_str().unwrap());
        println!("Run `bsky_tui generate` to generate a config file");
        return;
    }

    let config = AppConfig::load().unwrap();
    if let Err(e) = config.check_required_fields() {
        println!("Config file error: {}", e);
        return;
    }

    console_subscriber::init();
    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<IoEvent>(100);

    let app = Arc::new(tokio::sync::Mutex::new(App::new(sync_io_tx.clone())));
    let app_ui = Arc::clone(&app);

    tokio::spawn(async move {
        let mut handler = IoAsyncHandler::new(app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_io_event(io_event).await;
        }
    });

    start_ui(&app_ui, config.skip_splash, get_splash(config.splash_path))
        .await
        .unwrap();
}

fn config_command() -> Command {
    Command::new("config")
        .description("Generate config file")
        .alias("c")
        .action(|_| {
            if AppConfig::config_exists() {
                println!("Config file already exists");
                return;
            }
            AppConfig::generate_config_file().unwrap();
            println!(
                "Config file generated at: {}",
                AppConfig::config_path().to_str().unwrap()
            );
        })
}
