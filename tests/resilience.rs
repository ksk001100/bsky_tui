use bsky_tui::{
    app::{App, AppReturn},
    inputs::key::Key,
};

#[tokio::test]
async fn closed_worker_channel_clears_loading_and_surfaces_an_error() {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);
    let mut app = App::new(tx);

    assert_eq!(app.do_action(Key::Char('r')).await, AppReturn::Continue);
    assert!(!app.is_loading());
    assert!(app
        .error()
        .is_some_and(|message| message.contains("background worker")));
}

#[tokio::test]
async fn ctrl_c_exits_even_before_initialization() {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(tx);
    assert_eq!(app.do_action(Key::Ctrl('c')).await, AppReturn::Exit);
}

#[tokio::test]
async fn q_and_escape_do_not_exit_before_initialization() {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(tx);

    assert_eq!(app.do_action(Key::Char('q')).await, AppReturn::Continue);
    assert_eq!(app.do_action(Key::Esc).await, AppReturn::Continue);
}
