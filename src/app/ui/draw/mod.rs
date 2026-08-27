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
use unicode_width::UnicodeWidthChar;

use crate::{
    app::moderation::{ModerationDecision, ModerationPrefs},
    app::state::{AppState, Tab},
    app::thread::ThreadEntry,
    bsky, utils,
};

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
    Paragraph::new(format!("Mode: {} (type `?` for help)", state.get_mode()))
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().fg(Color::White)))
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
    Paragraph::new(format!(
        "{message}\n\nRetry the action, or press q/Esc to quit."
    ))
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

pub fn help<'a>() -> Table<'a> {
    let rows = vec![
        // Header
        Row::new(vec![
            Cell::from("Mode").style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Tabs").style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Key").style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Description").style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        // Normal mode
        Row::new(vec![
            Cell::from("Normal Mode"),
            Cell::from("Home/Notifications/Search"),
            Cell::from("Tab"),
            Cell::from("Change tab"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("q, Ctrl+c, Esc"),
            Cell::from("Quit"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("r"),
            Cell::from("Reload list"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("?"),
            Cell::from("Show help popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("/"),
            Cell::from("Search mode"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications"),
            Cell::from("n"),
            Cell::from("New post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications"),
            Cell::from("N"),
            Cell::from("Reply selected post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("j, Ctrl+n, Down"),
            Cell::from("Select next post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Notifications/Search"),
            Cell::from("k, Ctrl+p, Up"),
            Cell::from("Select previous post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("l, Right"),
            Cell::from("Next page"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("h, Left"),
            Cell::from("Prev page"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("Enter"),
            Cell::from("Open selected post images"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("b"),
            Cell::from("Selected post open in browser"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("t"),
            Cell::from("Open selected post thread"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search/Thread"),
            Cell::from("e"),
            Cell::from("Open link or video embed"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Notifications"),
            Cell::from("Enter, b"),
            Cell::from("Open notification post in browser"),
        ]),
        Row::new(vec![
            Cell::from("Image Viewer"),
            Cell::from(""),
            Cell::from("h/l, Left/Right"),
            Cell::from("Show previous/next image"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("q, Esc"),
            Cell::from("Close image viewer"),
        ]),
        Row::new(vec![
            Cell::from("Thread"),
            Cell::from(""),
            Cell::from("j/k, Up/Down"),
            Cell::from("Move through ancestors and replies"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("b / q, Esc"),
            Cell::from("Open in browser / close thread"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("Ctrl+r"),
            Cell::from("Repost selected post (unrepost if already reposted)"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Home/Search"),
            Cell::from("Ctrl+l"),
            Cell::from("Like selected post (unlike if already liked)"),
        ]),
        // Post mode
        Row::new(vec![
            Cell::from("Post/Reply/Search"),
            Cell::from(""),
            Cell::from("Esc"),
            Cell::from("Return to normal mode"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Ctrl+s / Enter"),
            Cell::from("Send post/reply / insert newline"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Left, Ctrl+b"),
            Cell::from("Move cursor left"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Right, Ctrl+f"),
            Cell::from("Move cursor right"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Ctrl+a"),
            Cell::from("Move cursor to start of line"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Ctrl+e"),
            Cell::from("Move cursor to end of line"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Backspace, Ctrl+h"),
            Cell::from("Delete word"),
        ]),
        // Help mode
        Row::new(vec![
            Cell::from("Help"),
            Cell::from(""),
            Cell::from("Esc, q, ?"),
            Cell::from("Return to normal mode"),
        ]),
    ];

    Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(25),
            Constraint::Length(20),
            Constraint::Percentage(80),
        ],
    )
    .block(
        Block::default()
            .title("Help")
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
        .map(|feed| feed.post.data.clone())
        .collect::<Vec<_>>();
    post_list(
        posts,
        state.moderation(),
        width,
        format!(
            "Home ({}: {})",
            state.get_tl_current_cursor_index() + 1,
            state.get_timeline().unwrap_or_default().len()
        ),
    )
}

pub fn search_results(state: &AppState, width: u16) -> PostList<'static> {
    post_list(
        state.get_search_results().unwrap_or_default(),
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
            .title("Thread — j/k move, b browser, Esc close"),
    )
}

fn post_list(
    posts: Vec<PostViewData>,
    moderation: ModerationPrefs,
    width: u16,
    title: String,
) -> PostList<'static> {
    let inner_width = width.saturating_sub(4);
    let rendered = posts
        .iter()
        .map(|post| render_post(post, &moderation, inner_width))
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
    let mut lines = vec![Line::from(vec![
        Span::raw(prefix.clone()),
        Span::styled(
            format!("{display_name} "),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("@{handle} {duration}"),
            Style::default().fg(Color::Gray),
        ),
    ])];

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

    let attachment_urls = bsky::post_attachment_urls(post, moderation)
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
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
    let width = width.max(1);
    let mut output = Vec::new();
    for source_line in text.split('\n') {
        let mut line = String::new();
        let mut line_width = 0;
        for character in source_line.chars() {
            let character_width = character.width().unwrap_or(0);
            if line_width > 0 && line_width + character_width > width {
                output.push(std::mem::take(&mut line));
                line_width = 0;
            }
            line.push(character);
            line_width += character_width;
        }
        output.push(line);
    }
    if output.is_empty() {
        output.push(String::new());
    }
    output
}

pub fn notifications<'a>(state: &AppState) -> List<'a> {
    let notifications = state.get_notifications();
    let my_handle = state.get_handle();
    let size = crossterm::terminal::size().unwrap();
    let border = "=".repeat((size.0 - 4) as usize);

    let list_items: Vec<ListItem> = match notifications {
        Some(notifications) => notifications
            .iter()
            .map(|notification| {
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
                    _ => Span::from(""),
                };

                let duration_text = match DateTime::parse_from_rfc3339(datetime) {
                    Ok(dt) => utils::get_duration_string(dt, Utc::now().fixed_offset()),
                    Err(_) => "".into(),
                };

                // fixme
                let subject = match reason {
                    "reply" | "mention" | "quote" => {
                        if let Ok(r) = post::Record::try_from_unknown(notification.record.clone()) {
                            Some(r.text.clone())
                        } else {
                            None
                        }
                    }
                    "repost" => {
                        if let Ok(r) = repost::Record::try_from_unknown(notification.record.clone())
                        {
                            bsky::get_url(my_handle.clone(), r.subject.uri.clone())
                        } else {
                            None
                        }
                    }
                    "like" => like::Record::try_from_unknown(notification.record.clone())
                        .ok()
                        .and_then(|r| bsky::get_url(my_handle.clone(), r.subject.uri.clone())),
                    _ => None,
                };

                let reason_subject = match reason {
                    "reply" => "replied to your post",
                    "repost" => "reposted your post",
                    "like" => "liked your post",
                    "follow" => "followed you",
                    "mention" => "mentioned you",
                    "quote" => "quoted your post",
                    _ => "",
                };

                let item = match subject {
                    Some(subject) => vec![
                        Line::from(vec![
                            reason_icon,
                            Span::styled(
                                format!(" {} ", display_name),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                format!("@{} {}", handle, duration_text),
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
                            reason_icon,
                            Span::styled(
                                format!(" {display_name} "),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                format!("@{handle} {duration_text}"),
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
            .collect(),
        None => vec![],
    };

    List::new(list_items)
        .highlight_style(FOCUSED_POST_STYLE)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .padding(Padding::new(1, 1, 1, 1))
                .title(format!(
                    "Notifications (page {}, {} items)",
                    state.get_notifications_current_cursor_index() + 1,
                    state.get_notifications().unwrap_or_default().len()
                ))
                .border_type(BorderType::Plain),
        )
}

pub fn post_input<'a>(state: &AppState) -> Paragraph<'a> {
    let text = state.get_input().value().to_string();
    let remaining = 300_i64 - text.graphemes(true).count() as i64;
    Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .title(format!("New post — {remaining} remaining (Ctrl+s to send)"))
                .padding(Padding::new(1, 1, 1, 1)),
        )
}

pub fn search_input<'a>(state: &AppState) -> Paragraph<'a> {
    let text = state.get_input().value().to_string();
    Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .title("Search")
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

    let current_feed = state.get_current_feed();

    if current_feed.is_none() {
        return Paragraph::new("Error...");
    }

    let current_feed = current_feed.unwrap();
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
