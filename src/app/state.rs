use std::fmt;
use std::fmt::Display;
use std::sync::Arc;

use crate::bsky;
use atrium_api::app::bsky::feed::defs::FeedViewPost;
use ratatui::widgets::ListState;

#[derive(Clone)]
pub enum Mode {
    Normal,
    Post,
    Help,
}

impl Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Mode::Normal => "Normal",
            Mode::Post => "Post",
            Mode::Help => "Help",
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        agent: Arc<bsky::Agent>,
        feed: Option<Vec<FeedViewPost>>,
        input_text: String,
        input_cursor_position: usize,
        tl_list_state: ListState,
        tl_list_position: usize,
        mode: Mode,
    },
}

impl AppState {
    pub fn initialized(agent: bsky::Agent) -> Self {
        let agent = Arc::new(agent);
        Self::Initialized {
            agent,
            feed: None,
            input_text: String::new(),
            input_cursor_position: 0,
            tl_list_state: ListState::default().with_selected(Some(0)),
            tl_list_position: 0,
            mode: Mode::Normal,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn get_agent(&self) -> Option<Arc<bsky::Agent>> {
        if let Self::Initialized { agent, .. } = self {
            Some(agent.clone())
        } else {
            None
        }
    }

    pub fn get_input_text(&self) -> Option<String> {
        if let Self::Initialized { input_text, .. } = self {
            Some(input_text.clone())
        } else {
            None
        }
    }

    pub fn set_input_text(&mut self, text: String) {
        if let Self::Initialized { input_text, .. } = self {
            *input_text = text;
        }
    }

    pub fn insert_input_text(&mut self, c: char) {
        if let Self::Initialized {
            input_text,
            input_cursor_position,
            ..
        } = self
        {
            // let len = &c.to_string().len();
            // input_text.insert_str(*input_cursor_position, &c.to_string());
            // if input_cursor_position == &0 {
            //     input_text.push(c);
            //     *input_cursor_position = input_text.len() / 2;
            // } else if input_text.chars().count() == *input_cursor_position {
            //     input_text.push(c);
            //     *input_cursor_position = input_text.len() / 2;
            // } else {
            //     // input_text.insert(*input_cursor_position, c);
            //     let len = input_text.len() / 2;
            //     *input_text = input_text
            //         .get(..*input_cursor_position - 1)
            //         .unwrap_or("")
            //         .to_string()
            //         + &c.to_string()
            //         + &input_text
            //             .get(*input_cursor_position..)
            //             .unwrap_or("")
            //             .to_string();
            //     *input_cursor_position = len - *input_cursor_position + 1;
            // }

            // TODO: カーソル位置の調整がめんどいので後回し
            input_text.push(c);
            *input_cursor_position = input_text.len();

            // input_text.push(c);
        }
    }

    pub fn remove_input_text(&mut self) {
        if let Self::Initialized {
            input_text,
            input_cursor_position,
            ..
        } = self
        {
            if *input_cursor_position > 0 {
                // input_text.remove(*input_cursor_position - 1);
                input_text.pop();
                *input_cursor_position = input_text.len();
            }
        }
    }

    pub fn move_tl_scroll_up(&mut self) {
        if let Self::Initialized {
            tl_list_position,
            tl_list_state,
            ..
        } = self
        {
            if *tl_list_position > 0 {
                *tl_list_position -= 1;
                tl_list_state.select(Some(*tl_list_position));
            }
        }
    }

    pub fn move_tl_scroll_down(&mut self) {
        if let Self::Initialized {
            tl_list_position,
            tl_list_state,
            feed: Some(feed),
            ..
        } = self
        {
            if *tl_list_position < feed.len() - 1 {
                *tl_list_position += 1;
                tl_list_state.select(Some(*tl_list_position));
            }
        }
    }

    pub fn get_tl_list_position(&self) -> usize {
        if let Self::Initialized {
            tl_list_position, ..
        } = self
        {
            *tl_list_position
        } else {
            0
        }
    }

    pub fn move_input_cursor_left(&mut self) {
        if let Self::Initialized {
            input_cursor_position,
            ..
        } = self
        {
            if *input_cursor_position > 0 {
                *input_cursor_position -= 1;
            }
        }
    }

    pub fn move_input_cursor_right(&mut self) {
        if let Self::Initialized {
            input_text,
            input_cursor_position,
            ..
        } = self
        {
            if *input_cursor_position < input_text.len() {
                *input_cursor_position += 1;
            }
        }
    }

    pub fn set_feed(&mut self, f: Vec<FeedViewPost>) {
        if let Self::Initialized { feed, .. } = self {
            *feed = Some(f);
        }
    }

    pub fn get_feed(&self) -> Option<Vec<FeedViewPost>> {
        if let Self::Initialized { feed, .. } = self {
            feed.clone()
        } else {
            None
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if let Self::Initialized { mode: m, .. } = self {
            *m = mode;
        }
    }

    pub fn get_mode(&self) -> Mode {
        if let Self::Initialized { mode, .. } = self {
            mode.clone()
        } else {
            Mode::Normal
        }
    }

    pub fn is_normal_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Normal)
        } else {
            false
        }
    }

    pub fn is_post_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Post)
        } else {
            false
        }
    }

    pub fn is_help_mode(&self) -> bool {
        if let Self::Initialized { mode, .. } = self {
            matches!(mode, Mode::Help)
        } else {
            false
        }
    }

    pub fn get_input_cursor_position(&self) -> usize {
        if let Self::Initialized {
            input_cursor_position,
            ..
        } = self
        {
            *input_cursor_position
        } else {
            0
        }
    }

    pub fn get_tl_list_state(&self) -> ListState {
        if let Self::Initialized { tl_list_state, .. } = self {
            tl_list_state.clone()
        } else {
            ListState::default()
        }
    }

    pub fn get_current_feed(&self) -> Option<FeedViewPost> {
        if let Self::Initialized {
            feed,
            tl_list_position,
            ..
        } = self
        {
            feed.clone().and_then(|f| f.get(*tl_list_position).cloned())
        } else {
            None
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init
    }
}
