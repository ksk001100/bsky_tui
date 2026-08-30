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
    use crate::inputs::key::Key;

    #[test]
    fn messages_drive_the_model_through_one_update_entrypoint() {
        let mut app = App::new();

        assert_eq!(
            app.update(Message::KeyPressed(Key::Ctrl('c'))).control,
            AppReturn::Exit
        );
    }
}
