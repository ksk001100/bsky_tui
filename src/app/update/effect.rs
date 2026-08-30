//! Reducer for messages returned by the asynchronous effect runtime.

use super::super::{message::EffectMessage, state::Mode, App};

impl App {
    pub(in crate::app) fn apply_effect(&mut self, message: EffectMessage) {
        match message {
            EffectMessage::Finished { error } => {
                self.loaded();
                if let Some(error) = error {
                    self.set_error(error);
                } else {
                    self.clear_error();
                }
            }
            EffectMessage::RuntimeError(error) => self.set_error(error),
            EffectMessage::Initialized {
                agent,
                handle,
                did,
                moderation,
                ui_config,
            } => self.initialized(agent, handle, did, moderation, ui_config),
            EffectMessage::TimelineLoaded {
                posts,
                position,
                new_count,
                cursors,
                cursor_index,
                image_urls,
            } => {
                self.state
                    .set_timeline_preserving_position(Some(posts), position);
                self.state.set_active_feed_new_count(new_count);
                self.state.set_cursors(cursors);
                self.state.set_tl_current_cursor_index(cursor_index);
                self.defer_images(image_urls);
            }
            EffectMessage::FeedCatalogLoaded { catalog, open } => {
                self.set_feed_catalog(catalog, open)
            }
            EffectMessage::FeedSearchLoaded(results) => self.set_feed_search_results(results),
            EffectMessage::FeedActivated(feed) => self.state.activate_feed(feed),
            EffectMessage::ThreadClosed => self.state.close_thread(),
            EffectMessage::ComposerPreviewLoaded(preview) => self.set_composer_preview(preview),
            EffectMessage::ThreadLoaded {
                entries,
                image_urls,
            } => {
                self.state.set_thread(entries);
                self.defer_images(image_urls);
            }
            EffectMessage::InteractionsLoaded { kind, items } => self.set_interactions(kind, items),
            EffectMessage::UserSearchLoaded(items) => {
                self.state.set_mode(Mode::Normal);
                self.state.set_input(tui_input::Input::default());
                self.set_interactions(crate::io::InteractionKind::Users, items);
            }
            EffectMessage::ProfileLoaded {
                profile,
                image_urls,
            } => {
                self.state.open_profile(profile);
                self.set_interactions_closed();
                self.defer_images(image_urls);
            }
            EffectMessage::ProfileContentLoaded {
                section,
                content,
                image_urls,
            } => {
                self.state.set_profile_content(section, content);
                self.defer_images(image_urls);
            }
            EffectMessage::ProfileUpdated(profile) => self.state.open_profile(profile),
            EffectMessage::ComposerFinished => {
                self.state.set_mode(Mode::Normal);
                self.state.set_input(tui_input::Input::default());
            }
            EffectMessage::NotificationsLoaded {
                notifications,
                posts,
                cursors,
                cursor_index,
                image_urls,
            } => {
                self.state.set_notifications(Some(notifications));
                self.state.set_notification_posts(posts);
                self.defer_images(image_urls);
                self.state.set_notification_cursors(cursors);
                self.state
                    .set_notifications_current_cursor_index(cursor_index);
            }
            EffectMessage::NotificationSettingsLoaded(settings) => {
                self.notification_settings = Some(settings)
            }
            EffectMessage::SearchLoaded {
                query,
                posts,
                cursors,
                cursor_index,
                image_urls,
            } => {
                self.state.set_search_query(Some(query));
                self.state.set_search_results(Some(posts));
                self.state.set_search_cursors(cursors);
                self.state.set_search_current_cursor_index(cursor_index);
                self.state.set_tab(super::super::state::Tab::Search);
                self.state.move_search_scroll_top();
                self.defer_images(image_urls);
            }
            EffectMessage::FeaturePanelClosed => self.feature_panel = None,
            EffectMessage::FeatureRowsLoaded { title, rows, child } => {
                self.set_feature_rows(title, rows, child)
            }
            EffectMessage::ConversationLoaded { title, rows } => {
                if self.state.get_tab() == super::super::state::Tab::Messages {
                    self.open_message_conversation(title, rows);
                } else {
                    let child = self
                        .feature_panel()
                        .is_some_and(|panel| !panel.title.starts_with("Conversation ·"));
                    self.set_feature_rows(title, rows, child);
                }
            }
            EffectMessage::MessagesReplaced { title, rows } => self.set_message_rows(title, rows),
            EffectMessage::ExploreReplaced(rows) => self.set_explore_rows(rows),
            EffectMessage::UiConfigLoaded(config) => self.set_ui_config(config),
        }
    }
}
