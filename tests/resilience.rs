use bsky_tui::{
    app::{command::Command, message::Message, App, AppReturn},
    inputs::key::Key,
    io::IoEvent,
};

#[test]
fn retry_returns_a_command_without_performing_io() {
    let mut app = App::new();

    let update = app.update(Message::KeyPressed(Key::Char('r')));

    assert_eq!(update.control, AppReturn::Continue);
    assert!(matches!(
        update.commands.as_slice(),
        [Command::Io {
            event: IoEvent::Initialize,
            ..
        }]
    ));
    assert!(app.is_loading());
}

#[test]
fn ctrl_c_exits_even_before_initialization() {
    let mut app = App::new();
    assert_eq!(
        app.update(Message::KeyPressed(Key::Ctrl('c'))).control,
        AppReturn::Exit
    );
}

#[test]
fn q_and_escape_do_not_exit_before_initialization() {
    let mut app = App::new();

    assert_eq!(
        app.update(Message::KeyPressed(Key::Char('q'))).control,
        AppReturn::Continue
    );
    assert_eq!(
        app.update(Message::KeyPressed(Key::Esc)).control,
        AppReturn::Continue
    );
}
