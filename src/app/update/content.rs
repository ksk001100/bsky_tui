//! content update handlers.

use super::super::*;

impl App {
    pub(in crate::app) fn search_action(&mut self, key: Key) -> AppReturn {
        if self.state.get_search_query().is_none() {
            return self.explore_action(key);
        }
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
            Key::Esc => {
                self.state.set_search_query(None);
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::Discovery,
                )));
                AppReturn::Continue
            }
            Key::F5 => {
                self.dispatch(IoEvent::Search(SearchEvent::Reload));
                AppReturn::Continue
            }
            Key::Char('r') => {
                self.state.set_tab(Tab::Search);
                self.state.set_mode(state::Mode::Reply);
                AppReturn::Continue
            }
            Key::Char('X') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.start_quote_composer(&post);
                }
                AppReturn::Continue
            }
            Key::Ctrl('r') => {
                self.dispatch(IoEvent::SearchRepost);
                AppReturn::Continue
            }
            Key::Ctrl('l') => {
                self.dispatch(IoEvent::SearchLike);
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
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.state.move_search_scroll_down();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.state.move_search_scroll_up();
                AppReturn::Continue
            }
            Key::PageDown | Key::Ctrl('d') => {
                self.state.move_search_scroll_by(5);
                AppReturn::Continue
            }
            Key::PageUp | Key::Ctrl('u') => {
                self.state.move_search_scroll_by(-5);
                AppReturn::Continue
            }
            Key::Home => {
                self.state.move_search_top();
                AppReturn::Continue
            }
            Key::End => {
                self.state.move_search_bottom();
                AppReturn::Continue
            }
            Key::Char('y') | Key::Char('Y') | Key::Alt('y') => {
                self.copy_selected_post(key);
                AppReturn::Continue
            }
            Key::Char('o') => {
                if let Some(feed) = self.state.get_current_search_result() {
                    if let Some(id) = feed.uri.split('/').next_back() {
                        let handle = &feed.author.handle;
                        let url =
                            format!("https://bsky.app/profile/{}/post/{}", handle.as_str(), id);
                        self.open_url(url, "Could not open search result");
                    }
                }
                AppReturn::Continue
            }
            Key::Enter => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadThread(post.uri.clone()));
                }
                AppReturn::Continue
            }
            Key::Char('a') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadProfile(post.author.did.clone().into()));
                }
                AppReturn::Continue
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.request_post_moderation(&post, key);
                }
                AppReturn::Continue
            }
            Key::Char('D') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.request_delete(&post);
                }
                AppReturn::Continue
            }
            Key::Char('e') => {
                self.open_selected_embed(self.state.get_current_search_result());
                AppReturn::Continue
            }
            Key::Char('f') => {
                self.open_facet_viewer(self.state.get_current_search_result());
                AppReturn::Continue
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(post) = self.state.get_current_search_result() {
                    self.dispatch(IoEvent::LoadInteractions(
                        interaction_kind(key),
                        post.uri,
                        post.cid,
                    ));
                }
                AppReturn::Continue
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(self.state.get_current_search_result());
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
                match self.state.get_search_query() {
                    Some(_) => {
                        self.dispatch(IoEvent::Search(SearchEvent::Prev));
                    }
                    None => {
                        let query = self.state.get_input().value().to_string();
                        self.state.set_search_query(Some(query.clone()));
                        self.dispatch(IoEvent::Search(SearchEvent::Prev));
                    }
                }
                AppReturn::Continue
            }
            Key::Char('l') | Key::Right | Key::Char(']') => {
                match self.state.get_search_query() {
                    Some(_) => {
                        self.dispatch(IoEvent::Search(SearchEvent::Next));
                    }
                    None => {
                        let query = self.state.get_input().value().to_string();
                        self.state.set_search_query(Some(query.clone()));
                        self.dispatch(IoEvent::Search(SearchEvent::Next));
                    }
                }
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn explore_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Ctrl('c') => AppReturn::Exit,
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
            Key::F5 => {
                self.dispatch(IoEvent::Feature(FeatureEvent::Load(
                    feature_panel::FeatureSection::Discovery,
                )));
                AppReturn::Continue
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') => {
                self.explore.next();
                AppReturn::Continue
            }
            Key::Up | Key::Char('k') | Key::Ctrl('p') => {
                self.explore.previous();
                AppReturn::Continue
            }
            Key::Home => {
                self.explore.selected = 0;
                AppReturn::Continue
            }
            Key::End => {
                self.explore.selected = self.explore.rows.len().saturating_sub(1);
                AppReturn::Continue
            }
            Key::Enter | Key::Char('o') => {
                match self.explore.selected_row().map(|row| row.target.clone()) {
                    Some(feature_panel::FeatureTarget::Topic(topic)) => {
                        self.state.set_search_query(Some(topic.clone()));
                        self.dispatch(IoEvent::Search(SearchEvent::Load(topic)));
                    }
                    Some(feature_panel::FeatureTarget::Actor(actor)) => {
                        self.dispatch(IoEvent::LoadProfile(actor));
                    }
                    _ => {}
                }
                AppReturn::Continue
            }
            Key::Tab => {
                self.state.set_next_tab();
                self.dispatch(IoEvent::LoadTimeline(TimelineEvent::Reload));
                AppReturn::Continue
            }
            _ => AppReturn::Continue,
        }
    }

    pub(in crate::app) fn profile_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.state.close_profile(),
            Key::Char('?') | Key::F1 => self.open_help(),
            Key::Down | Key::Char('j') | Key::Ctrl('n') => self.state.move_profile_down(),
            Key::Up | Key::Char('k') | Key::Ctrl('p') => self.state.move_profile_up(),
            Key::PageDown | Key::Ctrl('d') => self.state.move_profile_by(5),
            Key::PageUp | Key::Ctrl('u') => self.state.move_profile_by(-5),
            Key::Home => self.state.move_profile_top(),
            Key::End => self.state.move_profile_bottom(),
            Key::Char('y') | Key::Char('Y') | Key::Alt('y') => self.copy_selected_post(key),
            Key::Left | Key::Char('h') => {
                if let Some(profile) = self.state.get_profile() {
                    let section = profile.section.previous();
                    if section != profile.section {
                        self.dispatch(IoEvent::LoadProfileSection(section));
                    }
                }
            }
            Key::Right | Key::Char('l') => {
                if let Some(profile) = self.state.get_profile() {
                    let section = profile.section.next();
                    if section != profile.section {
                        self.dispatch(IoEvent::LoadProfileSection(section));
                    }
                }
            }
            Key::Char('F') => self.dispatch(IoEvent::ToggleFollow),
            Key::Char('X') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.start_quote_composer(&feed.post);
                }
            }
            Key::Char('D') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.request_delete(&feed.post);
                }
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(profile) = self.state.get_profile() {
                    self.request_profile_moderation(&profile.details, key);
                }
            }
            Key::Char('g') | Key::Char('G') => {
                if let Some(profile) = self.state.get_profile() {
                    let kind = if key == Key::Char('g') {
                        InteractionKind::Followers
                    } else {
                        InteractionKind::Follows
                    };
                    self.dispatch(IoEvent::LoadConnections(
                        kind,
                        profile.details.did.clone().into(),
                    ));
                }
            }
            Key::Char('t') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.dispatch(IoEvent::LoadThread(feed.post.uri.clone()));
                }
            }
            Key::Char('a') => {
                if let Some(feed) = self.state.get_current_profile_post() {
                    self.dispatch(IoEvent::LoadProfile(feed.post.author.did.clone().into()));
                }
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(
                    self.state
                        .get_current_profile_post()
                        .map(|feed| feed.post.data.clone()),
                );
            }
            Key::Enter | Key::Char('o') => {
                let url = self
                    .state
                    .get_current_profile_post()
                    .and_then(|feed| {
                        bsky::get_url(feed.post.author.handle.clone(), feed.post.uri.clone())
                    })
                    .or_else(|| self.state.get_current_profile_item().map(|item| item.url));
                if let Some(url) = url {
                    self.open_url(url, "Could not open profile item");
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn thread_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.state.close_thread(),
            Key::Char('?') | Key::F1 => self.open_help(),
            Key::Down | Key::Char('j') | Key::Ctrl('n') => self.state.move_thread_down(),
            Key::Up | Key::Char('k') | Key::Ctrl('p') => self.state.move_thread_up(),
            Key::PageDown => self.state.move_thread_by(5),
            Key::PageUp => self.state.move_thread_by(-5),
            Key::Home => self.state.move_thread_top(),
            Key::End => self.state.move_thread_bottom(),
            Key::Char('y') | Key::Char('Y') | Key::Alt('y') => self.copy_selected_post(key),
            Key::Char('o') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    if let Some(url) = bsky::get_url(post.author.handle.clone(), post.uri.clone()) {
                        self.open_url(url, "Could not open thread post");
                    }
                }
            }
            Key::Char('e') => {
                self.open_selected_embed(self.state.get_current_thread_post());
            }
            Key::Char('f') => {
                self.open_facet_viewer(self.state.get_current_thread_post());
            }
            Key::Char('a') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.dispatch(IoEvent::LoadProfile(post.author.did.clone().into()));
                }
            }
            Key::Char('i') | Key::Char(' ') => {
                self.open_post_images(self.state.get_current_thread_post());
            }
            Key::Char('X') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.start_quote_composer(&post);
                }
            }
            Key::Char('D') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.request_delete(&post);
                }
            }
            Key::Char('m') | Key::Char('B') | Key::Char('!') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.request_post_moderation(&post, key);
                }
            }
            Key::Char('H') => {
                let selected = self.state.get_current_thread_post();
                let did = self.state.get_did();
                let root = self
                    .state
                    .get_thread()
                    .into_iter()
                    .filter_map(|entry| entry.post().cloned())
                    .find(|post| Some(post.author.did.clone()) == did);
                if let (Some(root), Some(reply)) = (root, selected) {
                    if root.uri != reply.uri {
                        self.dispatch(IoEvent::Feature(FeatureEvent::ToggleHiddenReply {
                            root: Box::new(root),
                            reply: reply.uri,
                        }));
                    } else {
                        self.set_error("Select a reply beneath one of your posts".into());
                    }
                } else {
                    self.set_error("Hide reply is only available on your own thread".into());
                }
            }
            Key::Char('M') => {
                if let Some(root) = self
                    .state
                    .get_thread()
                    .into_iter()
                    .find_map(|entry| entry.post().cloned())
                {
                    let muted = config::AppConfig::load()
                        .map(|config| config.muted_threads.contains(&root.uri))
                        .unwrap_or(false);
                    self.dispatch(IoEvent::Feature(FeatureEvent::ToggleThreadMute {
                        root: root.uri,
                        muted,
                    }));
                }
            }
            Key::Ctrl('d') => {
                if let Some(quote) = self.state.get_current_thread_post() {
                    if let Some((post, author)) = bsky::quoted_post(&quote) {
                        if Some(author) == self.state.get_did() {
                            self.dispatch(IoEvent::Feature(FeatureEvent::DetachQuote {
                                post,
                                quote: quote.uri,
                            }));
                        } else {
                            self.set_error("Only quotes of your own post can be detached".into());
                        }
                    } else {
                        self.set_error("The selected post is not a quote post".into());
                    }
                }
            }
            Key::Char('L') | Key::Char('R') | Key::Char('Q') => {
                if let Some(post) = self.state.get_current_thread_post() {
                    self.dispatch(IoEvent::LoadInteractions(
                        interaction_kind(key),
                        post.uri,
                        post.cid,
                    ));
                }
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub(in crate::app) fn open_selected_embed(
        &mut self,
        post: Option<atrium_api::app::bsky::feed::defs::PostViewData>,
    ) {
        let Some(url) = post.as_ref().and_then(bsky::post_embed_url) else {
            self.set_error("The selected post has no external link or video".to_owned());
            return;
        };
        self.open_url(url, "Could not open embedded content");
    }

    pub(in crate::app) fn open_post_images(
        &mut self,
        post: Option<atrium_api::app::bsky::feed::defs::PostViewData>,
    ) {
        let moderation = self.state.moderation();
        let (urls, alt_texts) = post.map_or_else(
            || (Vec::new(), Vec::new()),
            |post| {
                (
                    bsky::post_attachment_fullsize_urls(&post, &moderation),
                    bsky::post_attachment_alt_texts(&post),
                )
            },
        );
        self.open_image_viewer(urls, alt_texts);
    }

    pub(in crate::app) fn start_quote_composer(
        &mut self,
        post: &atrium_api::app::bsky::feed::defs::PostViewData,
    ) {
        self.state.set_input(Input::new(format!(
            "!quote {} | {}\n",
            post.uri,
            post.cid.as_ref()
        )));
        self.state.set_mode(state::Mode::Post);
    }

    pub(in crate::app) fn request_post_moderation(
        &mut self,
        post: &atrium_api::app::bsky::feed::defs::PostViewData,
        key: Key,
    ) {
        if key == Key::Char('!') {
            self.open_report_prompt(feature_panel::ReportSubject::Record {
                uri: post.uri.clone(),
                cid: post.cid.clone(),
            });
            return;
        }
        self.pending_confirmation = Some(actor_moderation_action(&post.author, key));
    }

    pub(in crate::app) fn request_profile_moderation(
        &mut self,
        profile: &atrium_api::app::bsky::actor::defs::ProfileViewDetailedData,
        key: Key,
    ) {
        if key == Key::Char('!') {
            self.open_report_prompt(feature_panel::ReportSubject::Account(profile.did.clone()));
            return;
        }
        let viewer = profile.viewer.as_ref();
        self.pending_confirmation = Some(match key {
            Key::Char('m') => ModerationAction::MuteActor {
                did: profile.did.clone(),
                muted: viewer.and_then(|viewer| viewer.muted).unwrap_or(false),
            },
            Key::Char('B') => ModerationAction::BlockActor {
                did: profile.did.clone(),
                blocking_uri: viewer.and_then(|viewer| viewer.blocking.clone()),
            },
            _ => ModerationAction::ReportActor(profile.did.clone()),
        });
    }

    pub(in crate::app) fn open_report_prompt(&mut self, subject: feature_panel::ReportSubject) {
        let mut panel =
            feature_panel::FeaturePanel::loading(feature_panel::FeatureSection::Moderation);
        panel.title = "Report content".into();
        panel.prompt = Some(feature_panel::FeaturePrompt {
            label: "Report".into(),
            help: "spam|rude|sexual|violation|misleading|other | details".into(),
            action: feature_panel::FeaturePromptAction::Report { subject },
            input: Input::new("other | ".into()),
        });
        self.feature_panel = Some(panel);
    }

    pub(in crate::app) fn request_delete(
        &mut self,
        post: &atrium_api::app::bsky::feed::defs::PostViewData,
    ) {
        if Some(post.author.did.clone()) != self.state.get_did() {
            self.set_error("Only your own posts can be deleted".to_owned());
            return;
        }
        self.pending_delete = Some(post.uri.clone());
    }

    pub(in crate::app) fn confirmation_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Char('y') | Key::Char('Y') | Key::Enter => {
                if let Some(uri) = self.pending_delete.take() {
                    self.dispatch(IoEvent::DeletePost(uri));
                } else if let Some(action) = self.pending_confirmation.take() {
                    self.dispatch(IoEvent::Moderate(action));
                }
            }
            Key::Char('n') | Key::Char('N') | Key::Esc | Key::Char('q') => {
                self.pending_confirmation = None;
                self.pending_delete = None;
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn confirmation_message(&self) -> Option<String> {
        self.pending_delete
            .as_ref()
            .map(|uri| format!("Delete your post {uri}? This cannot be undone."))
            .or_else(|| {
                self.pending_confirmation
                    .as_ref()
                    .map(ModerationAction::confirmation)
            })
    }
}
