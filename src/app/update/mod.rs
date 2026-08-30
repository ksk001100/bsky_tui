//! Elm-style update handlers, grouped by feature domain.

use super::command::Command;
use super::{message::Message, App, AppReturn};

mod composer;
mod content;
mod features;
mod notifications;
mod timeline;

pub struct Update {
    pub control: AppReturn,
    pub commands: Vec<Command>,
}
mod effect;

impl App {
    /// Apply one message to the model.
    pub fn update(&mut self, message: Message) -> Update {
        let control = match message {
            Message::KeyPressed(key) => self.handle_key(key),
            Message::Mouse(mouse) => self.handle_mouse(mouse),
            Message::Tick => self.update_on_tick(),
            Message::Effect(message) => {
                self.apply_effect(*message);
                AppReturn::Continue
            }
        };
        Update {
            control,
            commands: self.take_commands(),
        }
    }

    /// Initialize the model and return its first command.
    pub fn init(&mut self) -> Update {
        self.dispatch(crate::io::IoEvent::Initialize);
        Update {
            control: AppReturn::Continue,
            commands: self.take_commands(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{config::UiConfig, moderation::ModerationPrefs};
    use crate::inputs::key::Key;

    #[test]
    fn messages_drive_the_model_through_one_update_entrypoint() {
        let mut app = App::new();

        assert_eq!(
            app.update(Message::KeyPressed(Key::Ctrl('c'))).control,
            AppReturn::Exit
        );
    }

    #[test]
    fn failed_effect_finishes_loading_and_preserves_the_error() {
        let mut app = App::new();
        let _ = app.init();

        let update = app.update(Message::effect(
            crate::app::message::EffectMessage::Finished {
                error: Some("timeline failed".into()),
            },
        ));

        assert_eq!(update.control, AppReturn::Continue);
        assert!(update.commands.is_empty());
        assert!(!app.is_loading());
        assert_eq!(app.error(), Some("timeline failed"));
    }

    #[test]
    fn repeated_input_while_loading_does_not_emit_duplicate_io() {
        let mut app = App::new();
        let first = app.update(Message::KeyPressed(Key::Char('r')));
        let second = app.update(Message::KeyPressed(Key::Char('r')));

        assert!(matches!(first.commands.as_slice(), [Command::Io { .. }]));
        assert!(second.commands.is_empty());
        assert!(app.is_loading());
    }

    async fn initialized_app() -> App {
        let agent = bsky_sdk::BskyAgent::builder()
            .build()
            .await
            .expect("test agent should build without network access");
        let handle =
            atrium_api::types::string::Handle::new("alice.test".into()).expect("valid test handle");
        let did =
            atrium_api::types::string::Did::new("did:plc:alice".into()).expect("valid test DID");
        let mut app = App::new();
        let update = app.update(Message::effect(
            crate::app::message::EffectMessage::Initialized {
                agent,
                handle,
                did,
                moderation: ModerationPrefs::default(),
                ui_config: UiConfig::default(),
            },
        ));
        assert!(update.commands.is_empty());
        app
    }

    #[tokio::test]
    async fn empty_timeline_page_is_a_valid_reducer_result() {
        let mut app = initialized_app().await;

        let update = app.update(Message::effect(
            crate::app::message::EffectMessage::TimelineLoaded {
                posts: Vec::new(),
                position: 99,
                new_count: 0,
                cursors: vec![None],
                cursor_index: 0,
                image_urls: Vec::new(),
            },
        ));

        assert!(update.commands.is_empty());
        assert_eq!(app.state().get_timeline(), Some(Vec::new()));
        assert_eq!(app.state().get_tl_list_position(), 0);
        assert_eq!(app.state().get_tl_list_state().selected(), None);
        assert_eq!(app.state().get_cursors(), vec![None]);
    }

    #[tokio::test]
    async fn timeline_images_are_emitted_as_a_command_without_io() {
        let mut app = initialized_app().await;
        let urls = vec!["https://cdn.example/one.jpg".to_owned()];

        let update = app.update(Message::effect(
            crate::app::message::EffectMessage::TimelineLoaded {
                posts: Vec::new(),
                position: 0,
                new_count: 0,
                cursors: vec![None],
                cursor_index: 0,
                image_urls: urls.clone(),
            },
        ));

        assert!(matches!(
            update.commands.as_slice(),
            [Command::LoadImages(command_urls)] if command_urls == &urls
        ));
    }
}
