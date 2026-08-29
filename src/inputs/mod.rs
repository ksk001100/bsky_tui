pub mod events;
pub mod key;

use crossterm::event::MouseEvent;

use self::key::Key;

pub enum InputEvent {
    Input(Key),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}
