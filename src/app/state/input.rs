//! input state operations.

use super::*;

impl AppState {
    pub fn get_did(&self) -> Option<Did> {
        if let Self::Initialized { did, .. } = self {
            Some(did.clone())
        } else {
            None
        }
    }

    pub fn get_input(&self) -> Input {
        if let Self::Initialized { input, .. } = self {
            input.clone()
        } else {
            Input::default()
        }
    }

    pub fn set_input(&mut self, i: Input) {
        if let Self::Initialized { input, .. } = self {
            *input = i;
        }
    }

    pub fn insert_input(&mut self, req: InputRequest) {
        if let Self::Initialized { input: i, .. } = self {
            i.handle(req);
        }
    }

    pub fn move_input_cursor_prev(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToPrevChar);
        }
    }

    pub fn move_input_cursor_next(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToNextChar);
        }
    }

    pub fn move_input_cursor_start(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToStart);
        }
    }

    pub fn move_input_cursor_end(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::GoToEnd);
        }
    }

    pub fn remove_input_prev(&mut self) {
        if let Self::Initialized { input, .. } = self {
            input.handle(InputRequest::DeletePrevChar);
        }
    }
}
