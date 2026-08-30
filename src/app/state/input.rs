//! Input and account identity state operations.

use super::*;

impl AppState {
    pub fn get_did(&self) -> Option<Did> {
        self.model().map(|model| model.session.did.clone())
    }

    pub fn get_input(&self) -> Input {
        self.model()
            .map(|model| model.composer.input.clone())
            .unwrap_or_default()
    }

    pub fn set_input(&mut self, input: Input) {
        if let Some(model) = self.model_mut() {
            model.composer.input = input;
        }
    }

    pub fn insert_input(&mut self, request: InputRequest) {
        if let Some(model) = self.model_mut() {
            model.composer.input.handle(request);
        }
    }

    pub fn move_input_cursor_prev(&mut self) {
        self.insert_input(InputRequest::GoToPrevChar);
    }
    pub fn move_input_cursor_next(&mut self) {
        self.insert_input(InputRequest::GoToNextChar);
    }
    pub fn move_input_cursor_start(&mut self) {
        self.insert_input(InputRequest::GoToStart);
    }
    pub fn move_input_cursor_end(&mut self) {
        self.insert_input(InputRequest::GoToEnd);
    }
    pub fn remove_input_prev(&mut self) {
        self.insert_input(InputRequest::DeletePrevChar);
    }
}
