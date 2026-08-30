//! Update handlers for the feature panel and its domain-specific sections.

use super::super::*;

mod action_menu;
mod chat;
mod discovery;
mod feeds;
mod lists;
mod moderation;
mod panel;
mod settings;
mod starter_packs;

impl App {
    pub(in crate::app) fn feature_panel_action(&mut self, key: Key) -> AppReturn {
        if self
            .feature_panel
            .as_ref()
            .and_then(|panel| panel.prompt.as_ref())
            .is_some()
        {
            return self.feature_prompt_action(key);
        }

        if self.select_feature_section(key) {
            return AppReturn::Continue;
        }

        let selected = self
            .feature_panel
            .as_ref()
            .and_then(feature_panel::FeaturePanel::selected_row)
            .cloned();
        match key {
            Key::Esc => {
                let parent = self
                    .feature_panel
                    .as_mut()
                    .and_then(|panel| panel.parent.take())
                    .map(|parent| *parent);
                if let Some(parent) = parent {
                    self.feature_panel = Some(parent);
                } else {
                    self.feature_panel = None;
                }
            }
            Key::Char('k') | Key::Up => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.previous();
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(panel) = self.feature_panel.as_mut() {
                    panel.next();
                }
            }
            Key::Enter | Key::Char('o') => self.open_feature_target(selected.clone()),
            _ => {
                let section = self.feature_panel.as_ref().map(|panel| panel.section);
                match section {
                    Some(feature_panel::FeatureSection::Lists) => {
                        self.lists_action(key, selected.clone())
                    }
                    Some(feature_panel::FeatureSection::StarterPacks) => {
                        self.starter_packs_action(key, selected.clone())
                    }
                    Some(feature_panel::FeatureSection::DirectMessages) => {
                        self.chat_action(key, selected.clone())
                    }
                    Some(feature_panel::FeatureSection::Moderation) => {
                        self.moderation_action(key, selected.clone())
                    }
                    Some(feature_panel::FeatureSection::Settings) => {
                        self.settings_action(key, selected.clone())
                    }
                    _ => {}
                }
                self.cross_domain_moderation_action(key, selected);
            }
        }
        AppReturn::Continue
    }

    fn select_feature_section(&mut self, key: Key) -> bool {
        let section = match key {
            Key::Char('1') => Some(feature_panel::FeatureSection::Lists),
            Key::Char('2') => Some(feature_panel::FeatureSection::StarterPacks),
            Key::Char('3') => {
                self.open_discovery();
                return true;
            }
            Key::Char('4') => {
                self.open_direct_messages();
                return true;
            }
            Key::Char('5') => Some(feature_panel::FeatureSection::Moderation),
            Key::Char('6') => Some(feature_panel::FeatureSection::Settings),
            _ => None,
        };
        if let Some(section) = section {
            self.feature_panel = Some(feature_panel::FeaturePanel::loading(section));
            self.dispatch(IoEvent::Feature(FeatureEvent::Load(section)));
            true
        } else {
            false
        }
    }
}
