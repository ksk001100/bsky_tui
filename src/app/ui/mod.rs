mod draw;
mod layout;
mod theme;

pub(crate) use draw::HELP_ROW_COUNT;

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::{state::Tab, App};

pub fn render<B>(f: &mut Frame, app: &mut App)
where
    B: Backend,
{
    let size = f.area();

    let main_chunks = layout::main(size);
    let header_chunks = layout::header(main_chunks[0]);
    let body_chunks = layout::body(main_chunks[1]);

    let accent = app.accent_color();
    let title = draw::title(accent);
    f.render_widget(title, header_chunks[0]);

    let mode = draw::mode(app.state(), accent);
    f.render_widget(mode, header_chunks[1]);

    let tabs = draw::tabs(app.state(), accent);
    f.render_widget(tabs, body_chunks[0]);

    if app.is_loading() {
        let popup = draw::loading();
        let area = layout::popup(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    } else if app.content_mode() == crate::app::state::Mode::Profile {
        render_profile(f, app, body_chunks[1]);
    } else if app.content_mode() == crate::app::state::Mode::Thread {
        let thread = draw::thread(app.state(), body_chunks[1].width, accent);
        let mut list_state = app.state.get_thread_list_state();
        list_state.select(Some(app.state.get_thread_list_position()));
        f.render_stateful_widget(thread, body_chunks[1], &mut list_state);
    } else {
        match app.state.get_tab() {
            Tab::Home => {
                let posts = draw::timeline(app.state(), body_chunks[1].width, accent);
                let mut list_state = app.state.get_tl_list_state();
                list_state.select(Some(app.state.get_tl_list_position()));
                f.render_stateful_widget(posts.widget, body_chunks[1], &mut list_state);
                render_post_images(f, app, &posts.layouts, body_chunks[1], list_state.offset());
            }
            Tab::Notifications => {
                let posts = draw::notifications(app.state(), body_chunks[1].width, accent);
                let mut list_state = app.state.get_notifications_list_state();
                list_state.select(Some(app.state.get_notifications_list_position()));
                f.render_stateful_widget(posts.widget, body_chunks[1], &mut list_state);
                render_post_images(f, app, &posts.layouts, body_chunks[1], list_state.offset());
            }
            Tab::Messages => {
                let panel = app.messages();
                let body = draw::messages(panel, accent);
                let mut list_state = ListState::default()
                    .with_selected((!panel.rows.is_empty()).then_some(panel.selected));
                f.render_stateful_widget(body, body_chunks[1], &mut list_state);
            }
            Tab::Search => {
                if app.state.get_search_query().is_some() {
                    let posts = draw::search_results(app.state(), body_chunks[1].width, accent);
                    let mut list_state = app.state.get_search_list_state();
                    list_state.select(Some(app.state.get_search_list_position()));
                    f.render_stateful_widget(posts.widget, body_chunks[1], &mut list_state);
                    render_post_images(f, app, &posts.layouts, body_chunks[1], list_state.offset());
                } else {
                    let panel = app.explore();
                    let body = draw::explore(panel, accent);
                    let mut list_state = ListState::default()
                        .with_selected((!panel.rows.is_empty()).then_some(panel.selected));
                    f.render_stateful_widget(body, body_chunks[1], &mut list_state);
                }
            }
        };
    }

    f.render_widget(draw::key_hints(app.key_hints(), accent), main_chunks[2]);

    if let Some(settings) = app.notification_settings.as_ref() {
        let area = layout::popup(88, 78, size);
        let mut state =
            ratatui::widgets::ListState::default().with_selected(Some(settings.category));
        f.render_widget(Clear, area);
        f.render_stateful_widget(
            draw::notification_settings(settings, accent),
            area,
            &mut state,
        );
    }

    if app.state.is_help_mode() {
        let popup = draw::help(accent);
        let area = layout::popup(90, 80, size);
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut app.help_table_state);
    }

    if app.state.is_post_mode() {
        let popup = draw::post_input(app.state());
        let area = layout::input_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        let (column, row) = app.input_cursor_position();
        f.set_cursor_position(Position::new(area.x + 2 + column, area.y + 2 + row));
    }

    if app.state.is_reply_mode() {
        let popup = draw::reply_input(app.state());
        let area = layout::reply_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        let (column, row) = app.input_cursor_position();
        f.set_cursor_position(Position::new(area.x + 2 + column, area.y + 6 + row));
    }

    if app.state.is_search_mode() {
        let popup = draw::search_input(app.state());
        let area = layout::input_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        f.set_cursor_position(Position::new(
            area.x + 2 + app.state.get_input().visual_cursor() as u16,
            area.y + 2,
        ));
    }

    if app.state.is_user_search_mode() {
        let popup = draw::user_search_input(app.state());
        let area = layout::input_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        f.set_cursor_position(Position::new(
            area.x + 2 + app.state.get_input().visual_cursor() as u16,
            area.y + 2,
        ));
    }

    if app.state.is_feed_search_mode() {
        let popup = draw::feed_search_input(app.state());
        let area = layout::input_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        f.set_cursor_position(Position::new(
            area.x + 2 + app.state.get_input().visual_cursor() as u16,
            area.y + 2,
        ));
    }

    if let Some((url, alt, index, total)) = app.current_image_viewer() {
        render_image_viewer(f, app, &url, &alt, index, total, size);
    }

    if let Some((facets, selected)) = app.current_facet_viewer() {
        let popup = draw::facets(facets, accent);
        let area = layout::popup(80, 60, size);
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut list_state);
    }

    if let Some((kind, items, selected)) = app.current_interactions() {
        let popup = draw::interactions(kind, items, accent);
        let area = layout::popup(80, 70, size);
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut list_state);
    }

    if let Some((items, selected)) = app.current_feed_viewer() {
        let popup = draw::feed_picker(items, accent);
        let area = layout::popup(85, 75, size);
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut list_state);
    }

    if let Some((items, selected)) = app.current_action_menu() {
        let area = layout::popup(55, 60, size);
        let rows = items.into_iter().map(ListItem::new).collect::<Vec<_>>();
        let widget = List::new(rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme::border(accent))
                    .title(" Actions "),
            )
            .highlight_style(theme::selected(accent));
        let mut state = ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(widget, area, &mut state);
    }

    if let Some(preview) = app.composer_preview() {
        let popup = draw::link_preview(preview);
        let area = layout::popup(75, 45, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }

    if let Some(panel) = app.feature_panel() {
        let area = layout::popup(92, 86, size);
        let items = if panel.rows.is_empty() {
            vec![ListItem::new("No items")]
        } else {
            panel
                .rows
                .iter()
                .map(|row| {
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                if row.unread { "● " } else { "  " },
                                Style::default().fg(accent),
                            ),
                            Span::styled(
                                row.title.clone(),
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(format!("  {}", row.detail)),
                    ])
                })
                .collect()
        };
        let widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme::border(accent))
                    .title(format!(" {} ", panel.title))
                    .title_bottom(" n new  a add  e edit  x delete  s save/subscribe  f feed  w write  q/Esc back "),
            )
            .highlight_style(theme::selected(accent));
        let mut state =
            ListState::default().with_selected((!panel.rows.is_empty()).then_some(panel.selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(widget, area, &mut state);

        if let Some(prompt) = panel.prompt.as_ref() {
            let prompt_area = layout::popup(78, 28, size);
            let text = format!("{}\n\n{}", prompt.help, prompt.input.value());
            f.render_widget(Clear, prompt_area);
            f.render_widget(
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(theme::border(accent))
                            .title(format!(" {} ", prompt.label)),
                    )
                    .wrap(Wrap { trim: false }),
                prompt_area,
            );
            f.set_cursor_position(Position::new(
                prompt_area.x + 1 + prompt.input.visual_cursor() as u16,
                prompt_area.y + 4,
            ));
        }
    }

    if app.state.get_tab() == Tab::Messages {
        if let Some(prompt) = app.messages().prompt.as_ref() {
            let prompt_area = layout::popup(78, 28, size);
            let text = format!("{}\n\n{}", prompt.help, prompt.input.value());
            f.render_widget(Clear, prompt_area);
            f.render_widget(
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(theme::border(accent))
                            .title(format!(" {} ", prompt.label)),
                    )
                    .wrap(Wrap { trim: false }),
                prompt_area,
            );
            f.set_cursor_position(Position::new(
                prompt_area.x + 1 + prompt.input.visual_cursor() as u16,
                prompt_area.y + 4,
            ));
        }
    }

    if let Some(message) = app.error() {
        let popup = draw::error(message);
        let area = layout::popup(70, 30, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }

    if let Some(message) = app.confirmation_message() {
        let popup = draw::confirmation(message);
        let area = layout::popup(70, 30, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}

fn render_profile(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let accent = app.accent_color();
    let Some(profile) = app.state.get_profile() else {
        return;
    };
    let compact = area.width < 72 || area.height < 20;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 7 } else { 12 }),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if compact {
            [
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Min(1),
            ]
        } else {
            [
                Constraint::Length(16),
                Constraint::Percentage(25),
                Constraint::Min(24),
            ]
        })
        .split(chunks[0]);

    if !compact {
        render_profile_image(
            f,
            app,
            profile.details.avatar.as_deref(),
            header[0],
            "Avatar",
            accent,
        );
        render_profile_image(
            f,
            app,
            profile.details.banner.as_deref(),
            header[1],
            "Banner",
            accent,
        );
    }
    f.render_widget(draw::profile_header(&profile, accent), header[2]);
    f.render_widget(draw::profile_tabs(&profile, accent), chunks[1]);

    let mut list_state = profile.list_state;
    match &profile.content {
        crate::app::profile::ProfileContent::Posts(_) => {
            let posts =
                draw::profile_posts(&profile, app.state.moderation(), chunks[2].width, accent);
            f.render_stateful_widget(posts.widget, chunks[2], &mut list_state);
            render_post_images(f, app, &posts.layouts, chunks[2], list_state.offset());
        }
        crate::app::profile::ProfileContent::Items(_) => {
            f.render_stateful_widget(
                draw::profile_items(&profile, accent),
                chunks[2],
                &mut list_state,
            );
        }
    }
}

fn render_profile_image(
    f: &mut Frame,
    app: &mut App,
    url: Option<&str>,
    area: ratatui::layout::Rect,
    title: &str,
    accent: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border(accent))
        .title(format!(" {title} "));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if app.images_enabled() {
        if let Some(url) = url {
            if let Some(protocol) = app.image_mut(url) {
                f.render_stateful_widget(
                    StatefulImage::default().resize(Resize::Fit(None)),
                    inner,
                    protocol,
                );
                return;
            }
        }
    }
    f.render_widget(
        Paragraph::new("(none/loading)").alignment(Alignment::Center),
        inner,
    );
}

fn render_post_images(
    f: &mut Frame,
    app: &mut App,
    layouts: &[draw::PostLayout],
    area: ratatui::layout::Rect,
    offset: usize,
) {
    if !app.images_enabled() || area.width < 45 {
        return;
    }
    for placement in draw::image_placements(layouts, area, offset) {
        if let Some(protocol) = app.image_mut(&placement.url) {
            f.render_stateful_widget(
                StatefulImage::default().resize(Resize::Fit(None)),
                placement.area,
                protocol,
            );
        }
    }
}

fn render_image_viewer(
    f: &mut Frame,
    app: &mut App,
    url: &str,
    alt: &str,
    index: usize,
    total: usize,
    screen: ratatui::layout::Rect,
) {
    let area = layout::popup(90, 90, screen);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border(app.accent_color()))
        .title(format!(
            " Image {index}/{total}  h/← previous  l/→ next  q/Esc close "
        ));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let image_bounds = ratatui::layout::Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(3),
    );
    let alt_area = ratatui::layout::Rect::new(
        inner.x,
        image_bounds.bottom(),
        inner.width,
        inner.height.saturating_sub(image_bounds.height),
    );
    let image_area = app.centered_image_area(url, image_bounds);
    if let (Some(image_area), Some(protocol)) = (image_area, app.image_mut(url)) {
        f.render_stateful_widget(
            StatefulImage::default().resize(Resize::Fit(None)),
            image_area,
            protocol,
        );
    } else {
        f.render_widget(
            Paragraph::new("Loading image...").alignment(Alignment::Center),
            image_bounds,
        );
    }
    let alt = if alt.trim().is_empty() {
        "Alt text: (not provided)".to_owned()
    } else {
        format!("Alt text: {alt}")
    };
    f.render_widget(
        Paragraph::new(alt).wrap(ratatui::widgets::Wrap { trim: true }),
        alt_area,
    );
}

pub fn render_splash<B>(f: &mut Frame, splash_text: String)
where
    B: Backend,
{
    let size = f.area();
    let popup = draw::splash(splash_text);
    let area = layout::popup(60, 60, size);
    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
