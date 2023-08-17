use std::sync::Arc;

use seahorse::{App as SeahorseApp, Context};

use bsky_tui::{
    app::App,
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
        .run(std::env::args().collect());
}

#[tokio::main]
async fn action(_c: &Context) {
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

    start_ui(&app_ui).await.unwrap();
}
