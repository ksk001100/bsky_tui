use crate::app::state::{AppState, Tab};
use crate::bsky;
use atrium_api::records::Record;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, List, ListItem, Padding, Paragraph, Row, Table, Tabs,
};

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

pub fn loading<'a>() -> Paragraph<'a> {
    Paragraph::new("Loading...")
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().fg(Color::White)))
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
            Cell::from("Timeline/Notifications"),
            Cell::from("Tab"),
            Cell::from("Change tab"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("q, Ctrl+c, Esc"),
            Cell::from("Quit"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("r"),
            Cell::from("Reload list"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("?"),
            Cell::from("Show help popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("n"),
            Cell::from("Show new post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("n"),
            Cell::from("Show new post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("j, Ctrl+n, Down"),
            Cell::from("Select next post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline/Notifications"),
            Cell::from("k, Ctrl+p, Up"),
            Cell::from("Select previous post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Timeline"),
            Cell::from("Enter"),
            Cell::from("Selected post open in browser"),
        ]),
        // Post mode
        Row::new(vec![
            Cell::from("Post Mode"),
            Cell::from(""),
            Cell::from("Esc"),
            Cell::from("Return to normal mode"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Enter"),
            Cell::from("Send post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from("Backspace, Ctrl+h"),
            Cell::from("Delete word"),
        ]),
        // Help mode
        Row::new(vec![
            Cell::from("Help Mode"),
            Cell::from(""),
            Cell::from("Esc, q, ?"),
            Cell::from("Return to normal mode"),
        ]),
    ];

    Table::new(rows)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White).bg(Color::Black))
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Length(15),
            Constraint::Length(25),
            Constraint::Length(20),
            Constraint::Percentage(80),
        ])
        .column_spacing(1)
}

pub fn timeline<'a>(state: &AppState) -> List<'a> {
    let feeds = state.get_feeds();
    let size = crossterm::terminal::size().unwrap();
    let border = "=".repeat((size.0 - 4) as usize);

    let list_items: Vec<ListItem> = match feeds {
        Some(feeds) => feeds
            .iter()
            .map(|feed| {
                let post = feed.post.clone();
                let text = if let Record::AppBskyFeedPost(r) = post.record {
                    r.text
                } else {
                    "None".into()
                };
                let display_name = post
                    .author
                    .display_name
                    .clone()
                    .unwrap_or_else(|| "".into());
                let handle = post.author.handle.clone();
                let reply_count = post.reply_count.unwrap_or(0);
                let repost_count = post.repost_count.unwrap_or(0);
                let like_count = post.like_count.unwrap_or(0);
                let item = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", display_name),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(format!("@{}", handle), Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(text),
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
                    Line::from(Span::styled(
                        border.clone(),
                        Style::default().fg(Color::Gray),
                    )),
                ];

                ListItem::new(item)
            })
            .collect(),
        None => vec![],
    };

    List::new(list_items)
        .highlight_style(Style::default().bg(Color::Blue))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .padding(Padding::new(1, 1, 1, 1))
                .title(format!(
                    "Timeline ({})",
                    state.get_feeds().unwrap_or(vec![]).len()
                ))
                .border_type(BorderType::Plain),
        )
}

pub fn notifications<'a>(state: &AppState) -> List<'a> {
    let notifications = state.get_notifications();
    let my_handle = state.get_handle().unwrap();
    let size = crossterm::terminal::size().unwrap();
    let border = "=".repeat((size.0 - 4) as usize);

    let list_items: Vec<ListItem> = match notifications {
        Some(notifications) => notifications
            .iter()
            .map(|notification| {
                let handle = notification.author.handle.clone();
                let display_name = notification
                    .author
                    .display_name
                    .clone()
                    .unwrap_or_else(|| "".into());
                let reason = notification.reason.clone();
                let datetime = notification.indexed_at.clone();
                let reason_icon = match reason.as_str() {
                    "reply" => Span::styled("↩", Style::default().fg(Color::Gray)),
                    "repost" => Span::styled("🔁", Style::default().fg(Color::Green)),
                    "like" => Span::styled("❤", Style::default().fg(Color::Red)),
                    "follow" => Span::styled("➕", Style::default().fg(Color::Blue)),
                    "mention" => Span::styled("🔔", Style::default().fg(Color::Yellow)),
                    "quote" => Span::styled("📣", Style::default().fg(Color::Magenta)),
                    _ => Span::from(""),
                };

                let subject = match (reason.as_str(), &notification.record) {
                    ("reply", Record::AppBskyFeedPost(r)) => Some(r.text.clone()),
                    ("repost", Record::AppBskyFeedRepost(r)) => {
                        bsky::get_url(my_handle.clone(), r.subject.uri.clone())
                    }
                    ("like", Record::AppBskyFeedLike(r)) => {
                        bsky::get_url(my_handle.clone(), r.subject.uri.clone())
                    }
                    ("mention", Record::AppBskyFeedPost(r)) => Some(r.text.clone()),
                    ("quote", Record::AppBskyFeedPost(r)) => Some(r.text.clone()),
                    _ => None,
                };

                let item = match subject {
                    Some(subject) => vec![
                        Line::from(vec![
                            reason_icon,
                            Span::styled(
                                format!(" {} ", display_name),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(format!("@{} ", handle), Style::default().fg(Color::Gray)),
                            Span::styled(datetime, Style::default().fg(Color::Gray)),
                        ]),
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
                                format!(" {} ", display_name),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(format!("@{} ", handle), Style::default().fg(Color::Gray)),
                            Span::styled(datetime, Style::default().fg(Color::Gray)),
                        ]),
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
        .highlight_style(Style::default().bg(Color::Blue))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .padding(Padding::new(1, 1, 1, 1))
                .title(format!(
                    "Notifications ({})",
                    state.get_notifications().unwrap_or(vec![]).len()
                ))
                .border_type(BorderType::Plain),
        )
}

pub fn post_input<'a>(state: &AppState) -> Paragraph<'a> {
    let text = state.get_input_text().unwrap_or_else(|| "".into());
    Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .title("New post")
                .padding(Padding::new(1, 1, 1, 1)),
        )
}

pub fn tabs<'a>(state: &AppState) -> Tabs<'a> {
    let titles = vec![Tab::Timeline, Tab::Notifications]
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
