use crate::app::state::AppState;
use atrium_api::records::Record;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, List, ListItem, Padding, Paragraph, Row, Table,
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

pub fn help<'a>() -> Table<'a> {
    // let key_style = Style::default().fg(Color::LightCyan);
    // let help_style = Style::default().fg(Color::Gray);
    //
    // let mut rows = vec![];
    // for action in actions.actions().iter() {
    //     let keys: Vec<String> = action.keys().iter().map(|k| k.to_string()).collect();
    //     let key = keys.join(", ");
    //     let row = Row::new(vec![
    //         Cell::from(Span::styled(key, key_style)),
    //         Cell::from(Span::styled(action.to_string(), help_style)),
    //     ]);
    //     rows.push(row);
    // }

    let rows = vec![
        // Normal mode
        Row::new(vec![
            Cell::from("Normal Mode"),
            Cell::from("q, Ctrl+c, Esc"),
            Cell::from("Quit"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("r"),
            Cell::from("Reload timeline"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("?"),
            Cell::from("Show help popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("n"),
            Cell::from("Show new post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("n"),
            Cell::from("Show new post popup"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("j, Down"),
            Cell::from("Select next post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("k, Up"),
            Cell::from("Select previous post"),
        ]),
        // Post mode
        Row::new(vec![
            Cell::from("Post Mode"),
            Cell::from("Esc"),
            Cell::from("Return to normal mode"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Enter"),
            Cell::from("Send post"),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from("Backspace, Ctrl+h"),
            Cell::from("Delete word"),
        ]),
        // Help mode
        Row::new(vec![
            Cell::from("Help Mode"),
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
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Percentage(80),
        ])
        .column_spacing(1)
}

pub fn timeline<'a>(state: &AppState) -> List<'a> {
    let feed = state.get_feed();
    let size = crossterm::terminal::size().unwrap();
    let border = "=".repeat((size.0 - 4) as usize);

    let list_items: Vec<ListItem> = match feed {
        Some(feed) => feed
            .iter()
            .map(|f| {
                let post = f.post.clone();
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
                            format!("reply: {}", reply_count),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(
                            format!(" repost: {}", repost_count),
                            Style::default().fg(Color::Green),
                        ),
                        Span::styled(
                            format!(" like: {}", like_count),
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
                    state.get_feed().unwrap_or(vec![]).len()
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
                .padding(Padding::new(1, 1, 2, 1)),
        )
}

pub fn body<'a>(state: &AppState) -> List<'a> {
    timeline(state)
}
