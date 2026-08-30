//! notifications update handlers.

use super::super::*;

impl App {
    pub(in crate::app) fn notifications_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::F5 => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Reload));
                AppReturn::Continue
            }
            Key::Char('?') | Key::F1 => {
                self.open_help();
                AppReturn::Continue
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::Search);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('u') => {
                self.state.set_mode(state::Mode::UserSearch);
                self.state.set_input(Input::default());
                AppReturn::Continue
            }
            Key::Char('1') => {
                self.state.cycle_notification_reason_filter();
                AppReturn::Continue
            }
            Key::Char('2') => {
                self.state.cycle_notification_sender_filter();
                AppReturn::Continue
            }
            Key::Char('3') => {
                self.state.cycle_notification_read_filter();
                AppReturn::Continue
            }
            Key::Char('p') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LoadNotificationSettings(
                        notification.author.did.clone(),
                        notification.author.handle.to_string(),
                    ));
                }
                AppReturn::Continue
            }
            Key::Char('f') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::ToggleNotificationFollow(
                        notification.author.did.clone(),
                    ));
                }
                AppReturn::Continue
            }
            Key::Char('L') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LikeNotificationAuthor(
                        notification.author.did.clone(),
                    ));
                }
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_notifications_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_notifications_scroll_up();
                AppReturn::Continue
            }
            Key::PageDown | Key::Ctrl('d') => {
                self.state.move_notifications_scroll_by(5);
                AppReturn::Continue
            }
            Key::PageUp | Key::Ctrl('u') => {
                self.state.move_notifications_scroll_by(-5);
                AppReturn::Continue
            }
            Key::Home => {
                self.state.move_notifications_top();
                AppReturn::Continue
            }
            Key::End => {
                self.state.move_notifications_bottom();
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(post) = self.state.get_current_notification_post() {
                    self.dispatch(IoEvent::LoadThread(post.uri.clone()));
                } else {
                    self.set_error(
                        "This notification does not refer to a post that can be opened".to_owned(),
                    );
                }
                AppReturn::Continue
            }
            Key::Char('o') => {
                let url = self
                    .state
                    .get_current_notification()
                    .and_then(|notification| {
                        self.state
                            .get_handle()
                            .and_then(|handle| bsky::notification_post_url(&notification, &handle))
                    });
                match url {
                    Some(url) => self.open_url(url, "Could not open notification"),
                    None => self.set_error(
                        "This notification does not refer to a post that can be opened".to_owned(),
                    ),
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(notification) = self.state.get_current_notification() {
                    self.dispatch(IoEvent::LoadProfile(notification.author.did.clone().into()));
                }
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                match self.state.get_tab() {
                    Tab::Home => {
                        self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload));
                    }
                    Tab::Notifications => {
                        self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Load));
                    }
                    Tab::Messages => {
                        self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                            feature_panel::FeatureSection::DirectMessages,
                        )));
                    }
                    Tab::Search => {}
                }
                AppReturn::Continue
            }
            Key::Char('h') | Key::Left | Key::Char('[') => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Prev));
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') => {
                self.dispatch(IoEvent::LoadNotifications(NotificationEvent::Next));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn notification_settings_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.notification_settings = None,
            Key::Up | Key::Char('k') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.previous();
                }
            }
            Key::Down | Key::Char('j') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.next();
                }
            }
            Key::Char(' ') | Key::Char('p') | Key::Char('i') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    match key {
                        Key::Char(' ') => settings.toggle_list(),
                        Key::Char('p') => settings.toggle_push(),
                        Key::Char('i') => settings.cycle_include(),
                        _ => {}
                    }
                    let preferences = settings.preferences.clone();
                    self.dispatch(IoEvent::SaveNotificationPreferences(Box::new(preferences)));
                }
            }
            Key::Char('v') => {
                if let Some(settings) = self.notification_settings.as_mut() {
                    settings.cycle_activity();
                    if let Some((subject, _, activity)) = settings.activity_subject.clone() {
                        self.dispatch(IoEvent::SaveActivitySubscription { subject, activity });
                    }
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }
}
