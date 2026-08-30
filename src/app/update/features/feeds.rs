use super::*;

impl App {
    pub(in crate::app) fn feed_viewer_action(&mut self, key: Key) -> AppReturn {
        match key {
            Key::Esc => self.feed_viewer = None,
            Key::Char('k') | Key::Up => {
                if let Some(viewer) = &mut self.feed_viewer {
                    viewer.index = viewer.index.saturating_sub(1);
                }
            }
            Key::Char('j') | Key::Down => {
                if let Some(viewer) = &mut self.feed_viewer {
                    if viewer.index + 1 < viewer.items.len() {
                        viewer.index += 1;
                    }
                }
            }
            Key::Enter => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    self.feed_viewer = None;
                    self.dispatch(IoEvent::SelectFeed(feed));
                }
            }
            Key::Char('s') => {
                if let Some(feed) = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned()
                {
                    if matches!(feed.kind, feed::FeedKind::Custom(_)) {
                        self.dispatch(IoEvent::ToggleSavedFeed(feed));
                    }
                }
            }
            Key::Char('!') => {
                let feed = self
                    .feed_viewer
                    .as_ref()
                    .and_then(|viewer| viewer.items.get(viewer.index))
                    .cloned();
                if let Some(feed::FeedDescriptor {
                    kind: feed::FeedKind::Custom(uri),
                    ..
                }) = feed
                {
                    self.feed_viewer = None;
                    self.open_report_prompt(feature_panel::ReportSubject::Feed(uri));
                }
            }
            Key::Char('/') => {
                self.state.set_mode(state::Mode::FeedSearch);
                self.state.set_input(Input::default());
            }
            _ => {}
        }
        AppReturn::Continue
    }

    pub fn set_feed_catalog(&mut self, catalog: Vec<feed::FeedDescriptor>, open: bool) {
        self.feed_catalog = catalog.clone();
        if open {
            self.feed_viewer = Some(FeedViewer {
                items: catalog,
                index: 0,
            });
        }
    }

    pub fn set_feed_search_results(&mut self, results: Vec<feed::FeedDescriptor>) {
        self.feed_viewer = Some(FeedViewer {
            items: results,
            index: 0,
        });
    }

    pub fn current_feed_viewer(&self) -> Option<(Vec<feed::FeedDescriptor>, usize)> {
        let viewer = self.feed_viewer.as_ref()?;
        Some((viewer.items.clone(), viewer.index))
    }
}
