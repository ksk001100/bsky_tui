//! Side effects emitted by the application update layer.

use super::feature_panel::FeatureSection;
use super::state::AppState;
use crate::io::IoEvent;

#[derive(Clone)]
pub struct EffectContext {
    pub state: AppState,
    pub feature_panel_open: bool,
    pub feature_panel_section: Option<FeatureSection>,
}

/// Work delegated to the asynchronous I/O runtime.
///
/// Keeping commands as data makes the state-transition boundary explicit and
/// allows future reducers to be tested without performing network I/O.
pub enum Command {
    Io {
        event: IoEvent,
        context: Box<EffectContext>,
    },
    LoadImages(Vec<String>),
    PollImages,
    OpenUrl {
        url: String,
        error_context: &'static str,
    },
    CopyToClipboard {
        value: String,
        label: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn commands_are_forwarded_to_the_effect_runtime() {
        let mut app = App::new();

        let update = app.init();

        assert!(matches!(
            update.commands.as_slice(),
            [Command::Io {
                event: IoEvent::Initialize,
                ..
            }]
        ));
        assert!(app.is_loading());
    }
}
