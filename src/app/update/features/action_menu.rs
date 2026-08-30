use super::*;

impl App {
    pub fn current_action_menu(&self) -> Option<(Vec<&'static str>, usize)> {
        let menu = self.action_menu.as_ref()?;
        Some((
            menu.items.iter().map(|(label, _)| *label).collect(),
            menu.index,
        ))
    }

    pub(in crate::app) fn action_menu_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.action_menu = None,
            Key::Up | Key::Char('k') => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = menu.index.saturating_sub(1);
                }
            }
            Key::Down | Key::Char('j') => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = (menu.index + 1).min(menu.items.len().saturating_sub(1));
                }
            }
            Key::Home => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = 0;
                }
            }
            Key::End => {
                if let Some(menu) = self.action_menu.as_mut() {
                    menu.index = menu.items.len().saturating_sub(1);
                }
            }
            Key::Enter => {
                let selected = self
                    .action_menu
                    .take()
                    .and_then(|menu| menu.items.get(menu.index).map(|(_, key)| *key));
                if let Some(key) = selected {
                    return match self.state.get_tab() {
                        Tab::Home => self.timeline_action(key),
                        Tab::Notifications => self.notifications_action(key),
                        Tab::Messages => self.messages_action(key),
                        Tab::Search => self.search_action(key),
                    };
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }
}
