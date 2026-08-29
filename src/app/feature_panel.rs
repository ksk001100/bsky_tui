use atrium_api::types::string::{AtIdentifier, Cid, Did};
use tui_input::Input;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureSection {
    Lists,
    StarterPacks,
    Discovery,
    DirectMessages,
    Moderation,
    Settings,
}

impl FeatureSection {
    pub fn title(self) -> &'static str {
        match self {
            Self::Lists => "Lists",
            Self::StarterPacks => "Starter Packs",
            Self::Discovery => "Discover",
            Self::DirectMessages => "Direct Messages",
            Self::Moderation => "Moderation & Safety",
            Self::Settings => "Settings & Accounts",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureTarget {
    List {
        uri: String,
        cid: Cid,
        purpose: String,
        owned: bool,
        muted: bool,
    },
    ListMember {
        list_uri: String,
        item_uri: String,
        actor: AtIdentifier,
    },
    StarterPack {
        uri: String,
        cid: Cid,
        owned: bool,
    },
    Actor(AtIdentifier),
    Topic(String),
    Conversation {
        id: String,
        muted: bool,
        members: Vec<Did>,
    },
    Message {
        convo_id: String,
        id: String,
        sender: Did,
    },
    Labeler(Did),
    LabelSetting {
        labeler: Did,
        label: String,
    },
    MutedWord(String),
    MutedThread(String),
    Account(String),
    Setting(SettingKey),
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKey {
    Images,
    DateFormat,
    Language,
    AccentColor,
    Keybindings,
    IncomingDm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureRow {
    pub title: String,
    pub detail: String,
    pub target: FeatureTarget,
    pub unread: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeaturePromptAction {
    CreateList,
    EditList { uri: String, purpose: String },
    AddListMember { list_uri: String },
    CreateStarterPack,
    EditStarterPack { uri: String },
    NewConversation,
    SendMessage { convo_id: String },
    AddMutedWord,
    AddLabeler,
    SetLabelVisibility { labeler: Did, label: String },
    Report { subject: ReportSubject },
    AddAccount,
    EditSetting(SettingKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportSubject {
    Account(Did),
    Record {
        uri: String,
        cid: Cid,
    },
    Feed(String),
    Conversation {
        convo_id: String,
        message_id: Option<String>,
        sender: Did,
    },
}

#[derive(Clone, Debug)]
pub struct FeaturePrompt {
    pub label: String,
    pub help: String,
    pub action: FeaturePromptAction,
    pub input: Input,
}

#[derive(Clone, Debug)]
pub struct FeaturePanel {
    pub section: FeatureSection,
    pub title: String,
    pub rows: Vec<FeatureRow>,
    pub selected: usize,
    pub prompt: Option<FeaturePrompt>,
    pub parent: Option<Box<FeaturePanel>>,
}

impl FeaturePanel {
    pub fn loading(section: FeatureSection) -> Self {
        Self {
            section,
            title: section.title().to_owned(),
            rows: Vec::new(),
            selected: 0,
            prompt: None,
            parent: None,
        }
    }

    pub fn selected_row(&self) -> Option<&FeatureRow> {
        self.rows.get(self.selected)
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn next(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    pub fn replace(&mut self, title: String, rows: Vec<FeatureRow>) {
        self.title = title;
        self.rows = rows;
        self.selected = 0;
    }

    pub fn child(&self, title: String, rows: Vec<FeatureRow>) -> Self {
        Self {
            section: self.section,
            title,
            rows,
            selected: 0,
            prompt: None,
            parent: Some(Box::new(self.clone())),
        }
    }
}

pub fn split_fields(value: &str, expected: usize) -> Result<Vec<String>, String> {
    let fields = value
        .split('|')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if fields.len() < expected || fields.iter().take(expected).any(String::is_empty) {
        return Err(format!(
            "expected {expected} non-empty fields separated by |"
        ));
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_selection_is_safe_for_empty_and_non_empty_rows() {
        let mut panel = FeaturePanel::loading(FeatureSection::Lists);
        panel.next();
        assert_eq!(panel.selected, 0);
        panel.rows.push(FeatureRow {
            title: "one".into(),
            detail: String::new(),
            target: FeatureTarget::Info,
            unread: false,
        });
        panel.next();
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn editor_fields_are_trimmed_and_validated() {
        assert_eq!(split_fields(" a | b ", 2).unwrap(), ["a", "b"]);
        assert!(split_fields("a|", 2).is_err());
    }
}
