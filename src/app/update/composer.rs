//! composer update handlers.

use super::super::*;

impl App {
    pub(in crate::app) fn search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Enter => {
                let query = self.state.get_input().value().to_string();
                if !query.is_empty() {
                    self.state.set_search_query(Some(query.clone()));
                    self.dispatch(IoEvent::Search(SearchEvent::Load(query)));
                    self.state.set_mode(state::Mode::Normal);
                    self.state.set_tab(Tab::Search);
                    self.state.set_input(Input::default());
                }
                AppReturn::Continue
            }
            Key::Left | Key::Ctrl('b') => {
                self.state.move_input_cursor_prev();
                AppReturn::Continue
            }
            Key::Right | Key::Ctrl('f') => {
                self.state.move_input_cursor_next();
                AppReturn::Continue
            }
            Key::Ctrl('a') => {
                self.state.move_input_cursor_start();
                AppReturn::Continue
            }
            Key::Ctrl('e') => {
                self.state.move_input_cursor_end();
                AppReturn::Continue
            }
            Key::Char(c) => {
                self.state.insert_input(InputRequest::InsertChar(c));
                AppReturn::Continue
            }
            Key::Backspace | Key::Ctrl('h') => {
                self.state.remove_input_prev();
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn user_search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
            }
            Key::Enter => {
                let query = self.state.get_input().value().trim().to_owned();
                if !query.is_empty() {
                    self.dispatch(IoEvent::SearchUsers(query));
                }
            }
            Key::Left | Key::Ctrl('b') => self.state.move_input_cursor_prev(),
            Key::Right | Key::Ctrl('f') => self.state.move_input_cursor_next(),
            Key::Ctrl('a') => self.state.move_input_cursor_start(),
            Key::Ctrl('e') => self.state.move_input_cursor_end(),
            Key::Char(c) => self.state.insert_input(InputRequest::InsertChar(c)),
            Key::Backspace | Key::Ctrl('h') => self.state.remove_input_prev(),
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn feed_search_input_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
            }
            Key::Enter => {
                let query = self.state.get_input().value().trim().to_owned();
                if !query.is_empty() {
                    self.dispatch(IoEvent::SearchFeeds(query));
                    self.state.set_mode(state::Mode::Normal);
                    self.state.set_input(Input::default());
                }
            }
            Key::Left | Key::Ctrl('b') => self.state.move_input_cursor_prev(),
            Key::Right | Key::Ctrl('f') => self.state.move_input_cursor_next(),
            Key::Ctrl('a') => self.state.move_input_cursor_start(),
            Key::Ctrl('e') => self.state.move_input_cursor_end(),
            Key::Char(c) => self.state.insert_input(InputRequest::InsertChar(c)),
            Key::Backspace | Key::Ctrl('h') => self.state.remove_input_prev(),
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn post_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Ctrl('s') => {
                self.dispatch(IoEvent::SendPost);
                AppReturn::Continue
            }
            Key::Ctrl('v') => {
                self.preview_composer_link();
                AppReturn::Continue
            }
            Key::Enter => {
                self.state.insert_input(InputRequest::InsertChar('\n'));
                AppReturn::Continue
            }
            Key::Left | Key::Ctrl('b') => {
                self.state.move_input_cursor_prev();
                AppReturn::Continue
            }
            Key::Right | Key::Ctrl('f') => {
                self.state.move_input_cursor_next();
                AppReturn::Continue
            }
            Key::Ctrl('a') => {
                self.state.move_input_cursor_start();
                AppReturn::Continue
            }
            Key::Ctrl('e') => {
                self.state.move_input_cursor_end();
                AppReturn::Continue
            }
            Key::Char(c) => {
                self.state.insert_input(InputRequest::InsertChar(c));
                AppReturn::Continue
            }
            Key::Backspace | Key::Ctrl('h') => {
                self.state.remove_input_prev();
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn reply_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => {
                self.state.set_mode(state::Mode::Normal);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Ctrl('s') => {
                if self.state.get_tab() == Tab::Search {
                    self.dispatch(IoEvent::SearchReply);
                } else {
                    self.dispatch(IoEvent::Reply);
                }
                AppReturn::Continue
            }
            Key::Ctrl('v') => {
                self.preview_composer_link();
                AppReturn::Continue
            }
            Key::Enter => {
                self.state.insert_input(InputRequest::InsertChar('\n'));
                AppReturn::Continue
            }
            Key::Left | Key::Ctrl('b') => {
                self.state.move_input_cursor_prev();
                AppReturn::Continue
            }
            Key::Right | Key::Ctrl('f') => {
                self.state.move_input_cursor_next();
                AppReturn::Continue
            }
            Key::Ctrl('a') => {
                self.state.move_input_cursor_start();
                AppReturn::Continue
            }
            Key::Ctrl('e') => {
                self.state.move_input_cursor_end();
                AppReturn::Continue
            }
            Key::Char(c) => {
                self.state.insert_input(InputRequest::InsertChar(c));
                AppReturn::Continue
            }
            Key::Backspace | Key::Ctrl('h') => {
                self.state.remove_input_prev();
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn help_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc | Key::Char('?') | Key::F1 => {
                self.close_help();
                AppReturn::Continue
            }
            Key::Char('j') | Key::Down => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some((selected + 1).min(ui::HELP_ROW_COUNT - 1)));
                AppReturn::Continue
            }
            Key::Char('k') | Key::Up => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some(selected.saturating_sub(1)));
                AppReturn::Continue
            }
            Key::PageDown => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some((selected + 10).min(ui::HELP_ROW_COUNT - 1)));
                AppReturn::Continue
            }
            Key::PageUp => {
                let selected = self.help_table_state.selected().unwrap_or(0);
                self.help_table_state
                    .select(Some(selected.saturating_sub(10)));
                AppReturn::Continue
            }
            Key::Home => {
                self.help_table_state.select(Some(0));
                AppReturn::Continue
            }
            Key::End => {
                self.help_table_state.select(Some(ui::HELP_ROW_COUNT - 1));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn image_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.image_viewer = None,
            Key::Char('h') | Key::Left => {
                if let Some(viewer) = &mut self.image_viewer {
                    viewer.previous();
                }
            }
            Key::Char('l') | Key::Right => {
                if let Some(viewer) = &mut self.image_viewer {
                    viewer.next();
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn facet_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.facet_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.facet_viewer {
                    viewer.previous();
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.facet_viewer {
                    viewer.next();
                }
            }
            Key::Enter | Key::Char('o') => {
                let url = self
                    .facet_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.facets.get(viewer.index))
                    .map(|facet| facet.url.clone());
                if let Some(url) = url {
                    self.open_url(url, "Could not open facet");
                    self.facet_viewer = None;
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn open_facet_viewer(
        &mut self,
        post: Option<atrium_api::app::bsky::feed::defs::PostViewData>,
    ) {
        let facets = post.as_ref().map_or_else(Vec::new, bsky::post_facets);
        self.facet_viewer = FacetViewer::new(facets);
        if self.facet_viewer.is_none() {
            self.set_error("The selected post has no URL, mention, or hashtag".to_owned());
        }
    }

    pub fn current_facet_viewer(&self) -> Option<(Vec<bsky::PostFacet>, usize)> {
        let viewer = self.facet_viewer.as_ref()?;
        Some((viewer.facets.clone(), viewer.index))
    }

    pub(in crate::app) fn interaction_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.interaction_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.interaction_viewer {
                    viewer.index = viewer.index.saturating_sub(1);
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.interaction_viewer {
                    if viewer.index + 1 < viewer.items.len() {
                        viewer.index += 1;
                    }
                }
            }
            Key::Enter | Key::Char('o') => {
                let selected = self
                    .interaction_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned();
                if let Some(actor) = selected.as_ref().and_then(|item| item.actor.clone()) {
                    self.dispatch(IoEvent::LoadProfile(actor));
                } else if let Some(url) = selected.map(|item| item.url) {
                    self.open_url(url, "Could not open interaction");
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }
}
