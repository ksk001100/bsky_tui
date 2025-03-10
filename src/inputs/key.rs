use std::fmt::{self, Display, Formatter};

use crossterm::event;

/// Represents an key.
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Key {
    /// Both Enter (or Return) and numpad Enter
    Enter,
    /// Tabulation key
    Tab,
    /// Backspace key
    Backspace,
    /// Escape key
    Esc,

    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,

    /// Insert key
    Ins,
    /// Delete key
    Delete,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,

    /// F0 key
    F0,
    /// F1 key
    F1,
    /// F2 key
    F2,
    /// F3 key
    F3,
    /// F4 key
    F4,
    /// F5 key
    F5,
    /// F6 key
    F6,
    /// F7 key
    F7,
    /// F8 key
    F8,
    /// F9 key
    F9,
    /// F10 key
    F10,
    /// F11 key
    F11,
    /// F12 key
    F12,
    Char(char),
    Ctrl(char),
    Alt(char),
    Unknown,
}

impl Key {
    /// If exit
    pub fn is_exit(&self) -> bool {
        matches!(self, Key::Ctrl('c') | Key::Char('q') | Key::Esc)
    }

    /// Returns the function key corresponding to the given number
    ///
    /// 1 -> F1, etc...
    ///
    /// # Panics
    ///
    /// If `n == 0 || n > 12`
    pub fn from_f(n: u8) -> Key {
        match n {
            0 => Key::F0,
            1 => Key::F1,
            2 => Key::F2,
            3 => Key::F3,
            4 => Key::F4,
            5 => Key::F5,
            6 => Key::F6,
            7 => Key::F7,
            8 => Key::F8,
            9 => Key::F9,
            10 => Key::F10,
            11 => Key::F11,
            12 => Key::F12,
            _ => panic!("unknown function key: F{}", n),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Key::Alt(' ') => write!(f, "<Alt+Space>"),
            Key::Ctrl(' ') => write!(f, "<Ctrl+Space>"),
            Key::Char(' ') => write!(f, "<Space>"),
            Key::Alt(c) => write!(f, "<Alt+{}>", c),
            Key::Ctrl(c) => write!(f, "<Ctrl+{}>", c),
            Key::Char(c) => write!(f, "<{}>", c),
            _ => write!(f, "<{:?}>", self),
        }
    }
}

impl From<event::KeyEvent> for Key {
    fn from(key_event: event::KeyEvent) -> Self {
        match key_event {
            event::KeyEvent {
                code: event::KeyCode::Esc,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Esc,
            event::KeyEvent {
                code: event::KeyCode::Backspace,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Backspace,
            event::KeyEvent {
                code: event::KeyCode::Left,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Left,
            event::KeyEvent {
                code: event::KeyCode::Right,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Right,
            event::KeyEvent {
                code: event::KeyCode::Up,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Up,
            event::KeyEvent {
                code: event::KeyCode::Down,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Down,
            event::KeyEvent {
                code: event::KeyCode::Home,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Home,
            event::KeyEvent {
                code: event::KeyCode::End,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::End,
            event::KeyEvent {
                code: event::KeyCode::PageUp,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::PageUp,
            event::KeyEvent {
                code: event::KeyCode::PageDown,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::PageDown,
            event::KeyEvent {
                code: event::KeyCode::Delete,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Delete,
            event::KeyEvent {
                code: event::KeyCode::Insert,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Ins,
            event::KeyEvent {
                code: event::KeyCode::F(n),
                kind: event::KeyEventKind::Press,
                ..
            } => Key::from_f(n),
            event::KeyEvent {
                code: event::KeyCode::Enter,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Enter,
            event::KeyEvent {
                code: event::KeyCode::Tab,
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Tab,

            // First check for char + modifier
            event::KeyEvent {
                code: event::KeyCode::Char(c),
                kind: event::KeyEventKind::Press,
                modifiers: event::KeyModifiers::ALT,
                ..
            } => Key::Alt(c),
            event::KeyEvent {
                code: event::KeyCode::Char(c),
                kind: event::KeyEventKind::Press,
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => Key::Ctrl(c),

            event::KeyEvent {
                code: event::KeyCode::Char(c),
                kind: event::KeyEventKind::Press,
                ..
            } => Key::Char(c),

            _ => Key::Unknown,
        }
    }
}
