use atrium_api::app::bsky::feed::{defs::PostViewData, like, post, repost};
use bsky_sdk::api::types::TryFromUnknown;
use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, Padding, Paragraph, Row, Table, Tabs,
    },
};
use unicode_segmentation::UnicodeSegmentation;

use crate::io::InteractionKind;
use crate::{
    app::feed::FeedDescriptor,
    app::moderation::{ModerationDecision, ModerationPrefs},
    app::profile::{ProfileContent, ProfileSection, ProfileState},
    app::state::{AppState, Tab},
    app::thread::ThreadEntry,
    bsky, utils,
};

pub fn notification_settings(
    settings: &crate::app::notifications::NotificationSettings,
) -> List<'static> {
    fn filterable(name: &str, include: &str, list: bool, push: bool) -> ListItem<'static> {
        ListItem::new(format!(
            "{name:<20} include:{include:<8} list:{} push:{}",
            on_off(list),
            on_off(push)
        ))
    }
    fn simple(name: &str, list: bool, push: bool) -> ListItem<'static> {
        ListItem::new(format!(
            "{name:<20}                  list:{} push:{}",
            on_off(list),
            on_off(push)
        ))
    }
    fn on_off(value: bool) -> &'static str {
        if value {
            "on "
        } else {
            "off"
        }
    }

    let p = &settings.preferences;
    let mut items = vec![
        filterable("like", &p.like.include, p.like.list, p.like.push),
        filterable("repost", &p.repost.include, p.repost.list, p.repost.push),
        filterable("follow", &p.follow.include, p.follow.list, p.follow.push),
        filterable(
            "mention",
            &p.mention.include,
            p.mention.list,
            p.mention.push,
        ),
        filterable("reply", &p.reply.include, p.reply.list, p.reply.push),
        filterable("quote", &p.quote.include, p.quote.list, p.quote.push),
        filterable(
            "like-via-repost",
            &p.like_via_repost.include,
            p.like_via_repost.list,
            p.like_via_repost.push,
        ),
        filterable(
            "repost-via-repost",
            &p.repost_via_repost.include,
            p.repost_via_repost.list,
            p.repost_via_repost.push,
        ),
        simple(
            "subscribed-post",
            p.subscribed_post.list,
            p.subscribed_post.push,
        ),
        simple(
            "starterpack-joined",
            p.starterpack_joined.list,
            p.starterpack_joined.push,
        ),
        simple("verified", p.verified.list, p.verified.push),
        simple("unverified", p.unverified.list, p.unverified.push),
        ListItem::new(format!(
            "{:<20} include:{:<8}         push:{}",
            "chat",
            p.chat.include,
            on_off(p.chat.push)
        )),
    ];
    if let Some((_, handle, activity)) = &settings.activity_subject {
        items.push(ListItem::new(""));
        items.push(ListItem::new(format!(
            "Activity for @{handle}: posts:{} replies:{}  (v cycles)",
            on_off(activity.post),
            on_off(activity.reply)
        )));
    }
    List::new(items).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default().borders(Borders::ALL).title(
            " Notification settings — Space list / p push / i audience / v activity / Esc close ",
        ),
    )
}

const FOCUSED_POST_STYLE: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::LightCyan)
    .add_modifier(Modifier::BOLD);

pub fn title<'a>() -> Paragraph<'a> {
    Paragraph::new(format!(
        "{} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ))
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Center)
    .block(Block::default().style(Style::default().fg(Color::White)))
}

pub fn mode<'a>(state: &AppState) -> Paragraph<'a> {
    Paragraph::new(format!("{}", state.get_mode()))
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().fg(Color::White)))
}

pub fn key_hints(hints: &str) -> Paragraph<'_> {
    Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
}

pub fn splash<'a>(text: String) -> Paragraph<'a> {
    Paragraph::new(text)
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(Block::default())
}

pub fn loading<'a>() -> Paragraph<'a> {
    Paragraph::new("Loading...")
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
}

pub fn error<'a>(message: &str) -> Paragraph<'a> {
    Paragraph::new(format!("{message}\n\nPress Esc to dismiss, or q to quit."))
        .style(Style::default().fg(Color::LightRed).bg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(
            Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 1, 1)),
        )
}

pub fn confirmation(message: String) -> Paragraph<'static> {
    Paragraph::new(format!(
        "{message}\n\nPress y/Enter to confirm, n/Esc to cancel."
    ))
    .wrap(ratatui::widgets::Wrap { trim: true })
    .block(
        Block::default()
            .title(" Confirm moderation action ")
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1)),
    )
}

pub fn link_preview(preview: bsky::LinkPreview) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(Span::styled(
            preview.title,
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(preview.description),
        Line::from(Span::styled(preview.url, Style::default().fg(Color::Gray))),
        Line::from(""),
        Line::from("Press any key to close preview"),
    ])
    .wrap(ratatui::widgets::Wrap { trim: true })
    .block(
        Block::default()
            .title(" Link card preview ")
            .borders(Borders::ALL),
    )
}

pub fn facets(facets: Vec<bsky::PostFacet>) -> List<'static> {
    let items = facets
        .into_iter()
        .map(|facet| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{}: ", facet.kind),
                        Style::default().fg(Color::LightCyan),
                    ),
                    Span::raw(facet.label),
                ]),
                Line::from(Span::styled(facet.url, Style::default().fg(Color::Gray))),
            ])
        })
        .collect::<Vec<_>>();
    List::new(items).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default()
            .title(" Links — ↑/↓ select, Enter open, Esc close ")
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1)),
    )
}

pub fn interactions(kind: InteractionKind, items: Vec<bsky::InteractionItem>) -> List<'static> {
    let title = match kind {
        InteractionKind::Likes => " Likes ",
        InteractionKind::Reposts => " Reposts ",
        InteractionKind::Quotes => " Quotes ",
        InteractionKind::Users => " User Search Results ",
        InteractionKind::Followers => " Followers ",
        InteractionKind::Follows => " Following ",
    };
    let rows = if items.is_empty() {
        vec![ListItem::new("(No results)")]
    } else {
        items
            .into_iter()
            .map(|item| ListItem::new(vec![Line::from(item.title), Line::from(item.subtitle)]))
            .collect()
    };
    List::new(rows).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default()
            .title(format!("{title}— ↑/↓ select, Enter open, Esc close "))
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1)),
    )
}

pub fn feed_picker(items: Vec<FeedDescriptor>) -> List<'static> {
    let rows = if items.is_empty() {
        vec![ListItem::new("(No feeds found)")]
    } else {
        items
            .into_iter()
            .map(|feed| {
                let flags = format!(
                    "{}{}",
                    if feed.pinned { "📌 " } else { "" },
                    if feed.saved { "★ " } else { "" }
                );
                ListItem::new(vec![
                    Line::from(Span::styled(
                        format!("{flags}{}", feed.name),
                        Style::default().fg(Color::LightCyan),
                    )),
                    Line::from(feed.description),
                ])
            })
            .collect()
    };
    List::new(rows).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default()
            .title(" Feeds — Enter select, / search, s save/unsave, Esc close ")
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1)),
    )
}

pub fn profile_header(profile: &ProfileState) -> Paragraph<'static> {
    let details = &profile.details;
    let display_name = details.display_name.clone().unwrap_or_default();
    let relationship = if details
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.following.as_ref())
        .is_some()
    {
        "Following — F to unfollow"
    } else {
        "Not following — F to follow"
    };
    let labels = details
        .labels
        .as_ref()
        .map(|labels| {
            labels
                .iter()
                .map(|label| format!("{}:{}", label.src.as_str(), label.val))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|labels| !labels.is_empty())
        .unwrap_or_else(|| "none".to_owned());
    let verification = details
        .verification
        .as_ref()
        .map(|verification| {
            let issuers = verification
                .verifications
                .iter()
                .filter(|item| item.is_valid)
                .map(|item| item.issuer.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Verification: account={} verifier={} issuers={}",
                verification.verified_status,
                verification.trusted_verifier_status,
                if issuers.is_empty() { "none" } else { &issuers }
            )
        })
        .unwrap_or_else(|| "Verification: none".to_owned());
    Paragraph::new(vec![
        Line::from(Span::styled(
            display_name,
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("@{}", details.handle.as_str())),
        Line::from(format!("DID: {}", details.did.as_str())),
        Line::from(format!(
            "{} posts   {} followers   {} following",
            details.posts_count.unwrap_or(0),
            details.followers_count.unwrap_or(0),
            details.follows_count.unwrap_or(0)
        )),
        Line::from(relationship),
        Line::from(format!("Labels: {labels}")),
        Line::from(verification),
        Line::from(details.description.clone().unwrap_or_default()),
    ])
    .wrap(ratatui::widgets::Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title(" Profile "))
}

pub fn profile_tabs(profile: &ProfileState) -> Tabs<'static> {
    let selected = ProfileSection::ALL
        .iter()
        .position(|section| *section == profile.section)
        .unwrap_or(0);
    Tabs::new(
        ProfileSection::ALL
            .iter()
            .map(|section| Line::from(section.label()))
            .collect::<Vec<_>>(),
    )
    .select(selected)
    .highlight_style(
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )
    .divider(" | ")
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" h/l switch section "),
    )
}

pub fn profile_posts(
    profile: &ProfileState,
    moderation: ModerationPrefs,
    width: u16,
) -> PostList<'static> {
    let posts = match &profile.content {
        ProfileContent::Posts(posts) => posts
            .iter()
            .map(|feed| (feed.post.data.clone(), bsky::feed_context_lines(feed)))
            .collect(),
        ProfileContent::Items(_) => Vec::new(),
    };
    post_list(posts, moderation, width, profile.section.label().to_owned())
}

pub fn profile_items(profile: &ProfileState) -> List<'static> {
    let items = match &profile.content {
        ProfileContent::Items(items) if !items.is_empty() => items
            .iter()
            .map(|item| {
                ListItem::new(vec![
                    Line::from(Span::styled(
                        item.title.clone(),
                        Style::default().fg(Color::LightCyan),
                    )),
                    Line::from(item.subtitle.clone()),
                ])
            })
            .collect(),
        _ => vec![ListItem::new("(No results)")],
    };
    List::new(items).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default().borders(Borders::ALL).title(format!(
            " {} — ↑/↓ select, Enter open, Esc back ",
            profile.section.label()
        )),
    )
}

const HELP_ROWS: &[(&str, &str, &str)] = &[
    ("Browse screens", "? / F1", "Open or close help"),
    ("", "Esc", "Cancel, close, or go back"),
    ("", "q", "Quit from any browsing screen"),
    ("", "Ctrl+C", "Quit immediately"),
    ("Main tabs", "Tab", "Switch Home → Notifications → Search"),
    ("", "↑/↓ or j/k", "Move selection"),
    ("", "←/→ or h/l", "Previous or next page"),
    ("", "[/] or PgUp/PgDn", "Previous or next page"),
    ("", "F5", "Reload the current list"),
    ("", "/", "Search posts"),
    ("", "u", "Search users"),
    (
        "Notifications",
        "1 / 2 / 3",
        "Filter by reason / sender / read state",
    ),
    ("", "Enter / o", "Open the related post"),
    ("", "a / f / L", "Profile / follow / like latest post"),
    ("", "p", "Open notification settings"),
    (
        "Notification settings",
        "↑/↓ or j/k",
        "Choose notification category",
    ),
    ("", "Space / p", "Toggle list / push notifications"),
    ("", "i", "Switch audience (all / following)"),
    ("", "v", "Cycle activity notifications for sender"),
    ("Home", "n / r", "New post / reply"),
    ("", "c", "Choose a feed"),
    ("Timeline posts", "Enter", "Open thread"),
    ("", "i / Space", "View attached images"),
    ("", "o", "Open in browser"),
    ("", "a", "Open author profile"),
    ("", "e / f", "Open embed / choose link or tag"),
    ("", "Ctrl+L / Ctrl+R", "Like / repost"),
    ("", "L / R / Q", "List likes / reposts / quotes"),
    ("", "X", "Quote post"),
    ("", "m / B / !", "Mute / block / report"),
    ("", "D", "Delete your own post"),
    ("Profile", "←/→ or h/l", "Switch profile section"),
    ("", "F", "Follow or unfollow"),
    ("", "g / G", "List followers / following"),
    ("", "Enter / o", "Open selected item"),
    ("Thread", "↑/↓ or j/k", "Move through the conversation"),
    ("", "o / a", "Open post / author"),
    ("Viewers", "←/→ or h/l", "Previous or next image"),
    ("", "↑/↓ or j/k", "Move through a list"),
    ("", "Enter / o", "Open selected item"),
    ("Composer", "Ctrl+S", "Send post or reply"),
    ("", "Enter", "Insert newline"),
    ("", "Ctrl+V", "Preview first !link card"),
    ("", "←/→", "Move cursor (Ctrl+B / Ctrl+F also work)"),
    ("", "Ctrl+A / Ctrl+E", "Start / end of line"),
    ("", "Backspace", "Delete previous character"),
    ("Search input", "Enter / Esc", "Run search / cancel"),
];

pub(crate) const HELP_ROW_COUNT: usize = HELP_ROWS.len();

pub fn help<'a>() -> Table<'a> {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let header = Row::new(["Context", "Keys", "Action"])
        .style(header_style)
        .bottom_margin(1);
    let rows = HELP_ROWS.iter().map(|(context, key, description)| {
        Row::new([
            Cell::from(*context),
            Cell::from(*key),
            Cell::from(*description),
        ])
    });

    Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(22),
            Constraint::Min(28),
        ],
    )
    .header(header)
    .row_highlight_style(FOCUSED_POST_STYLE)
    .highlight_symbol("▸ ")
    .block(
        Block::default()
            .title(" Keyboard help — ↑/↓ move, PgUp/PgDn jump, Esc close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .border_type(BorderType::Plain),
    )
    .column_spacing(1)
}

const AVATAR_HEIGHT: u16 = 4;
const MEDIA_HEIGHT: u16 = 8;

pub struct PostList<'a> {
    pub widget: List<'a>,
    pub layouts: Vec<PostLayout>,
}

#[derive(Clone)]
pub struct PostLayout {
    pub height: u16,
    pub avatar_url: Option<String>,
    pub attachment_urls: Vec<String>,
    pub avatar_width: u16,
    pub content_width: u16,
    pub media_row: Option<u16>,
}

pub struct ImagePlacement {
    pub url: String,
    pub area: Rect,
}

pub fn timeline(state: &AppState, width: u16) -> PostList<'static> {
    let posts = state
        .get_timeline()
        .unwrap_or_default()
        .into_iter()
        .map(|feed| (feed.post.data.clone(), bsky::feed_context_lines(&feed)))
        .collect::<Vec<_>>();
    post_list(
        posts,
        state.moderation(),
        width,
        format!(
            "{} (page {}, {} posts{})",
            state.get_active_feed().name,
            state.get_tl_current_cursor_index() + 1,
            state.get_timeline().unwrap_or_default().len(),
            match state.get_active_feed_new_count() {
                0 => String::new(),
                count => format!(", {count} new"),
            }
        ),
    )
}

pub fn search_results(state: &AppState, width: u16) -> PostList<'static> {
    post_list(
        state
            .get_search_results()
            .unwrap_or_default()
            .into_iter()
            .map(|post| (post, Vec::new()))
            .collect(),
        state.moderation(),
        width,
        format!(
            "Search Results ({}: {})",
            state.get_search_current_cursor_index() + 1,
            state.get_search_results().unwrap_or_default().len()
        ),
    )
}

pub fn thread(state: &AppState, width: u16) -> List<'static> {
    let moderation = state.moderation();
    let inner_width = width.saturating_sub(4).max(1) as usize;
    let items = state
        .get_thread()
        .into_iter()
        .map(|entry| {
            let (depth, lines) =
                match entry {
                    ThreadEntry::Placeholder { message, depth } => (depth, vec![message]),
                    ThreadEntry::Post {
                        post,
                        depth,
                        target,
                    } => {
                        let decision = moderation.decision(&post);
                        let marker = if target { "●" } else { "├" };
                        let display_name = post.author.display_name.clone().unwrap_or_default();
                        let handle = post.author.handle.as_str();
                        let text = match &decision {
                            ModerationDecision::HideContent { reason } => format!("[{reason}]"),
                            _ => post::Record::try_from_unknown(post.record.clone())
                                .map(|record| record.text.clone())
                                .unwrap_or_else(|_| "[Post record unavailable]".to_owned()),
                        };
                        let mut lines = vec![format!(
                            "{marker} {display_name} @{handle}  ↩ {} 🔁 {} ❤ {}",
                            post.reply_count.unwrap_or(0),
                            post.repost_count.unwrap_or(0),
                            post.like_count.unwrap_or(0)
                        )];
                        lines.extend(wrap_text(&text, inner_width.saturating_sub(depth * 2 + 2)));
                        if !matches!(&decision, ModerationDecision::HideContent { .. }) {
                            lines.extend(
                                bsky::post_embed_lines(&post)
                                    .into_iter()
                                    .map(|line| format!("│ {line}")),
                            );
                        }
                        if let ModerationDecision::WarnMedia { labels } = decision {
                            lines.push(format!("[Sensitive media hidden: {}]", labels.join(", ")));
                        }
                        lines.extend(bsky::post_attachment_alt_texts(&post).into_iter().map(
                            |alt| {
                                if alt.trim().is_empty() {
                                    "Alt: (not provided)".to_owned()
                                } else {
                                    format!("Alt: {alt}")
                                }
                            },
                        ));
                        (depth, lines)
                    }
                };
            let prefix = "  ".repeat(depth.min(20));
            ListItem::new(
                lines
                    .into_iter()
                    .map(|line| Line::from(format!("{prefix}{line}")))
                    .chain(std::iter::once(Line::from("")))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    List::new(items).highlight_style(FOCUSED_POST_STYLE).block(
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1))
            .title("Thread — j/k move, b browser, e embed, f facets, Esc close"),
    )
}

fn post_list(
    posts: Vec<(PostViewData, Vec<String>)>,
    moderation: ModerationPrefs,
    width: u16,
    title: String,
) -> PostList<'static> {
    let inner_width = width.saturating_sub(4);
    let rendered = posts
        .iter()
        .map(|(post, context)| render_post(post, context, &moderation, inner_width))
        .collect::<Vec<_>>();
    let layouts = rendered.iter().map(|(_, layout)| layout.clone()).collect();
    let items = rendered
        .into_iter()
        .map(|(lines, _)| ListItem::new(lines))
        .collect::<Vec<_>>();

    PostList {
        widget: List::new(items).highlight_style(FOCUSED_POST_STYLE).block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .padding(Padding::new(1, 1, 1, 1))
                .title(title)
                .border_type(BorderType::Plain),
        ),
        layouts,
    }
}

fn render_post(
    post: &PostViewData,
    context: &[String],
    moderation: &ModerationPrefs,
    inner_width: u16,
) -> (Vec<Line<'static>>, PostLayout) {
    let decision = moderation.decision(post);
    let avatar_width = match inner_width {
        50.. => 8,
        30.. => 6,
        18.. => 4,
        _ => 0,
    };
    let indent = avatar_width + u16::from(avatar_width > 0) * 2;
    let content_width = inner_width.saturating_sub(indent).max(1);
    let (mut text, created_at) =
        if let Ok(record) = post::Record::try_from_unknown(post.record.clone()) {
            (record.text.clone(), format!("{:?}+0000", record.created_at))
        } else {
            (String::new(), String::new())
        };
    let duration = DateTime::parse_from_str(&created_at, "%Y-%m-%dT%H:%M:%S%z")
        .map(|date| utils::get_duration_string(date, Utc::now().fixed_offset()))
        .unwrap_or_default();
    let display_name = post.author.display_name.clone().unwrap_or_default();
    let handle = post.author.handle.to_string();
    if let ModerationDecision::HideContent { reason } = &decision {
        text = format!("[{reason}]");
    }
    let prefix = " ".repeat(indent as usize);
    let mut lines = context
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::LightCyan),
            ))
        })
        .collect::<Vec<_>>();
    lines.push(Line::from(vec![
        Span::raw(prefix.clone()),
        Span::styled(
            format!("{display_name}{} ", bsky::verification_badge(post)),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("@{handle} {duration}"),
            Style::default().fg(Color::Gray),
        ),
    ]));

    for line in wrap_text(&text, content_width as usize) {
        lines.push(Line::from(format!("{prefix}{line}")));
    }

    if !matches!(&decision, ModerationDecision::HideContent { .. }) {
        for embed_line in bsky::post_embed_lines(post) {
            for line in wrap_text(&embed_line, content_width as usize) {
                lines.push(Line::from(format!("{prefix}│ {line}")));
            }
        }
    }

    let attachment_urls = if inner_width < 41 {
        Vec::new()
    } else {
        bsky::post_attachment_urls(post, moderation)
            .into_iter()
            .take(4)
            .collect::<Vec<_>>()
    };
    let media_row = if attachment_urls.is_empty() {
        None
    } else {
        lines.push(Line::from(prefix.clone()));
        let media_row = lines.len() as u16;
        lines.extend((0..MEDIA_HEIGHT).map(|_| Line::from(" ".repeat(inner_width as usize))));
        Some(media_row)
    };

    if let ModerationDecision::WarnMedia { labels } = &decision {
        lines.push(Line::from(format!(
            "{prefix}[Sensitive media hidden: {}]",
            labels.join(", ")
        )));
    }
    if !matches!(decision, ModerationDecision::HideContent { .. }) {
        for alt in bsky::post_attachment_alt_texts(post)
            .into_iter()
            .filter(|alt| !alt.trim().is_empty())
        {
            for line in wrap_text(&format!("Alt: {alt}"), content_width as usize) {
                lines.push(Line::from(format!("{prefix}{line}")));
            }
        }
    }

    lines.push(Line::from(vec![
        Span::raw(prefix),
        Span::styled(
            format!("↩ {}", post.reply_count.unwrap_or(0)),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            format!("   🔁 {}", post.repost_count.unwrap_or(0)),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!("   ❤ {}", post.like_count.unwrap_or(0)),
            Style::default().fg(Color::Red),
        ),
    ]));

    while lines.len() < AVATAR_HEIGHT as usize {
        lines.push(Line::from(" ".repeat(inner_width as usize)));
    }
    lines.push(Line::from(Span::styled(
        "=".repeat(inner_width as usize),
        Style::default().fg(Color::Gray),
    )));

    let layout = PostLayout {
        height: lines.len() as u16,
        avatar_url: decision
            .permits_media()
            .then(|| post.author.avatar.clone())
            .flatten(),
        attachment_urls,
        avatar_width,
        content_width,
        media_row,
    };
    (lines, layout)
}

pub fn image_placements(
    layouts: &[PostLayout],
    list_area: Rect,
    offset: usize,
) -> Vec<ImagePlacement> {
    let inner = Rect::new(
        list_area.x.saturating_add(2),
        list_area.y.saturating_add(2),
        list_area.width.saturating_sub(4),
        list_area.height.saturating_sub(4),
    );
    let mut placements = Vec::new();
    let mut y = inner.y;

    for layout in layouts.iter().skip(offset) {
        if y >= inner.bottom() {
            break;
        }

        if let Some(url) = &layout.avatar_url {
            let area = Rect::new(inner.x, y, layout.avatar_width, AVATAR_HEIGHT);
            if layout.avatar_width > 0 && area.bottom() <= inner.bottom() {
                placements.push(ImagePlacement {
                    url: url.clone(),
                    area,
                });
            }
        }

        if let Some(media_row) = layout.media_row {
            let media_area = Rect::new(
                inner.x + layout.avatar_width + u16::from(layout.avatar_width > 0) * 2,
                y.saturating_add(media_row),
                layout.content_width,
                MEDIA_HEIGHT,
            );
            if media_area.bottom() <= inner.bottom() {
                for (url, area) in layout
                    .attachment_urls
                    .iter()
                    .zip(media_grid(media_area, layout.attachment_urls.len()))
                {
                    placements.push(ImagePlacement {
                        url: url.clone(),
                        area,
                    });
                }
            }
        }

        y = y.saturating_add(layout.height);
    }
    placements
}

fn media_grid(area: Rect, count: usize) -> Vec<Rect> {
    let gap = u16::from(area.width >= 3);
    let left_width = area.width.saturating_sub(gap) / 2;
    let right_width = area.width.saturating_sub(gap + left_width);
    let top_height = area.height.saturating_sub(gap) / 2;
    let bottom_height = area.height.saturating_sub(gap + top_height);
    let left = area.x;
    let right = area.x + left_width + gap;
    let top = area.y;
    let bottom = area.y + top_height + gap;

    match count {
        0 => Vec::new(),
        1 => vec![area],
        2 => vec![
            Rect::new(left, top, left_width, area.height),
            Rect::new(right, top, right_width, area.height),
        ],
        3 => vec![
            Rect::new(left, top, left_width, area.height),
            Rect::new(right, top, right_width, top_height),
            Rect::new(right, bottom, right_width, bottom_height),
        ],
        _ => vec![
            Rect::new(left, top, left_width, top_height),
            Rect::new(right, top, right_width, top_height),
            Rect::new(left, bottom, left_width, bottom_height),
            Rect::new(right, bottom, right_width, bottom_height),
        ],
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;

    let width = width.max(1);
    let mut output = Vec::new();
    for source_line in text.split('\n') {
        let mut line = String::new();
        let mut line_width = 0;
        for grapheme in source_line.graphemes(true) {
            let grapheme_width = grapheme.width();
            if line_width > 0 && line_width + grapheme_width > width {
                output.push(std::mem::take(&mut line));
                line_width = 0;
            }
            line.push_str(grapheme);
            line_width += grapheme_width;
        }
        output.push(line);
    }
    if output.is_empty() {
        output.push(String::new());
    }
    output
}

pub fn notifications<'a>(state: &AppState) -> List<'a> {
    let notifications = state.notification_groups();
    let my_handle = state.get_handle();
    let size = crossterm::terminal::size().unwrap_or((80, 24));
    let border = "=".repeat(size.0.saturating_sub(4) as usize);

    let list_items: Vec<ListItem> = if notifications.is_empty() {
        vec![]
    } else {
        notifications
            .iter()
            .map(|group| {
                let notification = group.primary();
                let handle = notification.author.handle.to_string();
                let display_name = notification
                    .author
                    .display_name
                    .clone()
                    .unwrap_or_else(|| "".into());
                let reason = notification.reason.as_str();
                let datetime = notification.indexed_at.as_str();
                let reason_icon = match reason {
                    "reply" => Span::styled("↩", Style::default().fg(Color::Gray)),
                    "repost" => Span::styled("🔁", Style::default().fg(Color::Green)),
                    "like" => Span::styled("❤", Style::default().fg(Color::Red)),
                    "follow" => Span::styled("➕", Style::default().fg(Color::Blue)),
                    "mention" => Span::styled("🔔", Style::default().fg(Color::Yellow)),
                    "quote" => Span::styled("📣", Style::default().fg(Color::Magenta)),
                    "subscribed-post" => Span::styled("★", Style::default().fg(Color::Cyan)),
                    "like-via-repost" | "repost-via-repost" => {
                        Span::styled("↗", Style::default().fg(Color::Green))
                    }
                    "starterpack-joined" => Span::styled("✦", Style::default().fg(Color::Blue)),
                    "verified" => Span::styled("✓", Style::default().fg(Color::Blue)),
                    "unverified" => Span::styled("?", Style::default().fg(Color::Yellow)),
                    _ => Span::from(""),
                };

                let duration_text = match DateTime::parse_from_rfc3339(datetime) {
                    Ok(dt) => utils::get_duration_string(dt, Utc::now().fixed_offset()),
                    Err(_) => "".into(),
                };
                let unread = group.notifications.iter().any(|item| !item.is_read);
                let grouped = if group.notifications.len() > 1 {
                    format!(" +{} others", group.notifications.len() - 1)
                } else {
                    String::new()
                };
                let read_marker = if unread { "●" } else { "○" };

                // fixme
                let subject = match reason {
                    "reply" | "mention" | "quote" | "subscribed-post" => {
                        if let Ok(r) = post::Record::try_from_unknown(notification.record.clone()) {
                            Some(r.text.clone())
                        } else {
                            None
                        }
                    }
                    "repost" => {
                        if let Ok(r) = repost::Record::try_from_unknown(notification.record.clone())
                        {
                            my_handle
                                .clone()
                                .and_then(|handle| bsky::get_url(handle, r.subject.uri.clone()))
                        } else {
                            None
                        }
                    }
                    "like" => like::Record::try_from_unknown(notification.record.clone())
                        .ok()
                        .and_then(|r| {
                            my_handle
                                .clone()
                                .and_then(|handle| bsky::get_url(handle, r.subject.uri.clone()))
                        }),
                    _ => None,
                };

                let reason_subject = match reason {
                    "reply" => "replied to your post",
                    "repost" => "reposted your post",
                    "like" => "liked your post",
                    "follow" => "followed you",
                    "mention" => "mentioned you",
                    "quote" => "quoted your post",
                    "subscribed-post" => "made a new activity post",
                    "like-via-repost" => "liked your repost",
                    "repost-via-repost" => "reposted your repost",
                    "starterpack-joined" => "joined your starter pack",
                    "verified" => "verified your account",
                    "unverified" => "removed an account verification",
                    _ => "",
                };

                let item = match subject {
                    Some(subject) => vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{read_marker} "),
                                Style::default().fg(if unread {
                                    Color::Cyan
                                } else {
                                    Color::DarkGray
                                }),
                            ),
                            reason_icon,
                            Span::styled(
                                format!(" {} ", display_name),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                format!("@{} {}{}", handle, duration_text, grouped),
                                Style::default().fg(Color::Gray),
                            ),
                        ]),
                        Line::from(reason_subject),
                        Line::from(subject),
                        Line::from(Span::styled(
                            border.clone(),
                            Style::default().fg(Color::Gray),
                        )),
                    ],
                    None => vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{read_marker} "),
                                Style::default().fg(if unread {
                                    Color::Cyan
                                } else {
                                    Color::DarkGray
                                }),
                            ),
                            reason_icon,
                            Span::styled(
                                format!(" {display_name} "),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                format!("@{handle} {duration_text}{grouped}"),
                                Style::default().fg(Color::Gray),
                            ),
                        ]),
                        Line::from(reason_subject),
                        Line::from(Span::styled(
                            border.clone(),
                            Style::default().fg(Color::Gray),
                        )),
                    ],
                };

                ListItem::new(item)
            })
            .collect()
    };

    let filters = state.notification_filters();

    List::new(list_items)
        .highlight_style(FOCUSED_POST_STYLE)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .padding(Padding::new(1, 1, 1, 1))
                .title(format!(
                    "Notifications (page {}, {} groups) [reason:{} sender:{} read:{}]",
                    state.get_notifications_current_cursor_index() + 1,
                    notifications.len(),
                    filters.reason.label(),
                    filters.sender.label(),
                    filters.read.label(),
                ))
                .border_type(BorderType::Plain),
        )
}

pub fn post_input<'a>(state: &AppState) -> Paragraph<'a> {
    let text = state.get_input().value().to_string();
    let remaining = 300_i64 - text.graphemes(true).count() as i64;
    let summary = crate::app::composer::parse_drafts(&text)
        .map(|drafts| crate::app::composer::summary(&drafts))
        .unwrap_or_else(|_| "metadata incomplete".to_owned());
    Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .title(format!(
                    "New post — {remaining} remaining — {summary} — Ctrl+s send / Ctrl+v preview"
                ))
                .padding(Padding::new(1, 1, 1, 1)),
        )
}

pub fn search_input<'a>(state: &AppState) -> Paragraph<'a> {
    search_input_with_title(state, "Search posts")
}

pub fn user_search_input<'a>(state: &AppState) -> Paragraph<'a> {
    search_input_with_title(state, "Search users")
}

pub fn feed_search_input<'a>(state: &AppState) -> Paragraph<'a> {
    search_input_with_title(state, "Search custom feeds")
}

fn search_input_with_title<'a>(state: &AppState, title: &'a str) -> Paragraph<'a> {
    let text = state.get_input().value().to_string();
    Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .title(title)
                .padding(Padding::new(1, 1, 1, 1)),
        )
}

pub fn reply_input<'a>(state: &AppState) -> Paragraph<'a> {
    let text = state.get_input().value().to_string();
    let remaining = 300_i64 - text.graphemes(true).count() as i64;

    if state.get_tab() == Tab::Search {
        if let Some(search_result) = state.get_current_search_result() {
            let display_name = search_result
                .author
                .display_name
                .clone()
                .unwrap_or_else(|| "".into());
            let handle = search_result.author.handle.to_string();
            let parent_text =
                if let Ok(post) = post::Record::try_from_unknown(search_result.record.clone()) {
                    post.text.clone()
                } else {
                    "".to_string()
                };
            let reply_count = search_result.reply_count.unwrap_or(0);
            let repost_count = search_result.repost_count.unwrap_or(0);
            let like_count = search_result.like_count.unwrap_or(0);

            return Paragraph::new(vec![
                Line::from(format!("{display_name} @{handle}")),
                Line::from(parent_text),
                Line::from(vec![
                    Span::styled(
                        format!("↩ {}", reply_count),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("   🔁 {}", repost_count),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!("   ❤ {}", like_count),
                        Style::default().fg(Color::Red),
                    ),
                ]),
                Line::from(""),
                Line::from(text),
            ])
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .style(Style::default().fg(Color::White))
                    .borders(Borders::ALL)
                    .title(format!("Reply — {remaining} remaining (Ctrl+s to send)"))
                    .padding(Padding::new(1, 1, 1, 1)),
            );
        }
    }

    let Some(current_feed) = state.get_current_feed() else {
        return Paragraph::new("Select a post before replying");
    };
    let display_name = current_feed
        .post
        .author
        .display_name
        .clone()
        .unwrap_or_else(|| "".into());
    let handle = current_feed.post.author.handle.to_string();
    let parent_text =
        if let Ok(post) = post::Record::try_from_unknown(current_feed.post.record.clone()) {
            post.text.clone()
        } else {
            "".to_string()
        };
    let reply_count = current_feed.post.reply_count.unwrap_or(0);
    let repost_count = current_feed.post.repost_count.unwrap_or(0);
    let like_count = current_feed.post.like_count.unwrap_or(0);

    Paragraph::new(vec![
        Line::from(format!("{display_name} @{handle}")),
        Line::from(parent_text),
        Line::from(vec![
            Span::styled(
                format!("↩ {}", reply_count),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("   🔁 {}", repost_count),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("   ❤ {}", like_count),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(""),
        Line::from(text),
    ])
    .style(Style::default().fg(Color::White).bg(Color::Black))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .style(Style::default().fg(Color::White))
            .borders(Borders::ALL)
            .title(format!("Reply — {remaining} remaining (Ctrl+s to send)"))
            .padding(Padding::new(1, 1, 1, 1)),
    )
}

pub fn tabs<'a>(state: &AppState) -> Tabs<'a> {
    let titles: Vec<_> = [Tab::Home, Tab::Notifications, Tab::Search]
        .iter()
        .map(|t| format!("{}", t))
        .collect();

    Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .border_type(BorderType::Plain),
        )
        .select(state.get_tab() as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_wide_characters_by_terminal_width() {
        assert_eq!(wrap_text("あいう", 4), vec!["あい", "う"]);
        assert_eq!(wrap_text("abcd", 2), vec!["ab", "cd"]);
    }

    #[test]
    fn wrapping_preserves_emoji_combining_marks_and_rtl_order() {
        assert_eq!(wrap_text("e\u{301}x", 1), vec!["e\u{301}", "x"]);
        assert_eq!(wrap_text("👩‍💻a", 2), vec!["👩‍💻", "a"]);
        assert_eq!(wrap_text("שלום", 4).concat(), "שלום");
        assert_eq!(wrap_text("日本語", 4), vec!["日本", "語"]);
    }

    #[test]
    fn three_images_use_one_large_and_two_stacked_cells() {
        let cells = media_grid(Rect::new(10, 5, 21, 8), 3);
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0], Rect::new(10, 5, 10, 8));
        assert_eq!(cells[1], Rect::new(21, 5, 10, 3));
        assert_eq!(cells[2], Rect::new(21, 9, 10, 4));
    }

    #[test]
    fn image_placements_respect_list_padding_and_scroll_offset() {
        let layouts = vec![
            PostLayout {
                height: 5,
                avatar_url: Some("first".into()),
                attachment_urls: Vec::new(),
                avatar_width: 8,
                content_width: 40,
                media_row: None,
            },
            PostLayout {
                height: 5,
                avatar_url: Some("second".into()),
                attachment_urls: Vec::new(),
                avatar_width: 8,
                content_width: 40,
                media_row: None,
            },
        ];

        let placements = image_placements(&layouts, Rect::new(0, 0, 54, 10), 1);
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].url, "second");
        assert_eq!(placements[0].area, Rect::new(2, 2, 8, 4));
    }
}
