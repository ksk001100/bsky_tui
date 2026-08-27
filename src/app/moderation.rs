use std::collections::HashMap;

use atrium_api::{
    app::bsky::{actor::defs::PreferencesItem, feed::defs::PostViewData},
    types::Union,
};

#[derive(Clone, Debug, Default)]
pub struct ModerationPrefs {
    adult_content_enabled: bool,
    label_visibility: HashMap<(Option<String>, String), LabelVisibility>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelVisibility {
    Ignore,
    Warn,
    Hide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerationDecision {
    Show,
    WarnMedia { labels: Vec<String> },
    HideContent { reason: String },
}

impl ModerationDecision {
    pub fn permits_media(&self) -> bool {
        matches!(self, Self::Show)
    }
}

impl ModerationPrefs {
    pub fn from_api(preferences: &[Union<PreferencesItem>]) -> Self {
        let mut result = Self::default();
        for preference in preferences {
            let Union::Refs(preference) = preference else {
                continue;
            };
            match preference {
                PreferencesItem::AdultContentPref(pref) => {
                    result.adult_content_enabled = pref.enabled;
                }
                PreferencesItem::ContentLabelPref(pref) => {
                    let visibility = match pref.visibility.as_str() {
                        "ignore" => LabelVisibility::Ignore,
                        "hide" => LabelVisibility::Hide,
                        _ => LabelVisibility::Warn,
                    };
                    result.label_visibility.insert(
                        (
                            pref.labeler_did.as_ref().map(|did| did.as_str().to_owned()),
                            pref.label.clone(),
                        ),
                        visibility,
                    );
                }
                _ => {}
            }
        }
        result
    }

    pub fn decision(&self, post: &PostViewData) -> ModerationDecision {
        let viewer = post.author.viewer.as_deref();
        if viewer.and_then(|state| state.blocked_by).unwrap_or(false) {
            return ModerationDecision::HideContent {
                reason: "Post hidden: author has blocked you".to_owned(),
            };
        }
        if viewer.and_then(|state| state.blocking.as_ref()).is_some()
            || viewer
                .and_then(|state| state.blocking_by_list.as_ref())
                .is_some()
        {
            return ModerationDecision::HideContent {
                reason: "Post hidden: blocked author".to_owned(),
            };
        }
        if viewer.and_then(|state| state.muted).unwrap_or(false)
            || viewer
                .and_then(|state| state.muted_by_list.as_ref())
                .is_some()
        {
            return ModerationDecision::HideContent {
                reason: "Post hidden: muted author".to_owned(),
            };
        }

        let labels = post
            .author
            .labels
            .iter()
            .flatten()
            .chain(post.labels.iter().flatten())
            .filter(|label| !label.neg.unwrap_or(false))
            .map(|label| (label.src.as_str().to_owned(), label.val.clone()));
        self.decision_for_labels(labels)
    }

    fn decision_for_labels(
        &self,
        labels: impl IntoIterator<Item = (String, String)>,
    ) -> ModerationDecision {
        let mut warned = Vec::new();
        for (source, label) in labels {
            let visibility = self
                .label_visibility
                .get(&(Some(source), label.clone()))
                .or_else(|| self.label_visibility.get(&(None, label.clone())))
                .copied()
                .unwrap_or_else(|| self.default_visibility(&label));
            match visibility {
                LabelVisibility::Hide => {
                    return ModerationDecision::HideContent {
                        reason: format!("Post hidden by content label: {label}"),
                    };
                }
                LabelVisibility::Warn => warned.push(label),
                LabelVisibility::Ignore => {}
            }
        }
        if warned.is_empty() {
            ModerationDecision::Show
        } else {
            warned.sort();
            warned.dedup();
            ModerationDecision::WarnMedia { labels: warned }
        }
    }

    fn default_visibility(&self, label: &str) -> LabelVisibility {
        match label {
            // Adult-only media must never be rendered without an explicit preference.
            "porn" if !self.adult_content_enabled => LabelVisibility::Hide,
            "porn" | "sexual" | "nudity" | "graphic-media" | "gore" => LabelVisibility::Warn,
            // These system labels are informational rather than content warnings.
            "!no-unauthenticated" | "!warn" => LabelVisibility::Ignore,
            _ => LabelVisibility::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adult_media_is_hidden_until_enabled() {
        let prefs = ModerationPrefs::default();
        assert!(matches!(
            prefs.decision_for_labels([("labeler".into(), "porn".into())]),
            ModerationDecision::HideContent { .. }
        ));
    }

    #[test]
    fn sensitive_media_is_warned_and_not_permitted_for_rendering() {
        let prefs = ModerationPrefs::default();
        let decision = prefs.decision_for_labels([("labeler".into(), "nudity".into())]);
        assert_eq!(
            decision,
            ModerationDecision::WarnMedia {
                labels: vec!["nudity".into()]
            }
        );
        assert!(!decision.permits_media());
    }
}
