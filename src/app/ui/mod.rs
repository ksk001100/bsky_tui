mod draw;
mod layout;

pub(crate) use draw::HELP_ROW_COUNT;

use ratatui::{
    backend::Backend,
    layout::{Alignment, Position},
    widgets::{Block, Borders, Clear, Paragraph},
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

    let title = draw::title();
    f.render_widget(title, header_chunks[0]);

    let mode = draw::mode(app.state());
    f.render_widget(mode, header_chunks[1]);

    let tabs = draw::tabs(app.state());
    f.render_widget(tabs, body_chunks[0]);

    if app.is_loading() {
        let popup = draw::loading();
        let area = layout::popup(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    } else if app.state.is_thread_mode() {
        let thread = draw::thread(app.state(), body_chunks[1].width);
        let mut list_state = app.state.get_thread_list_state();
        list_state.select(Some(app.state.get_thread_list_position()));
        f.render_stateful_widget(thread, body_chunks[1], &mut list_state);
    } else {
        match app.state.get_tab() {
            Tab::Home => {
                let posts = draw::timeline(app.state(), body_chunks[1].width);
                let mut list_state = app.state.get_tl_list_state();
                list_state.select(Some(app.state.get_tl_list_position()));
                f.render_stateful_widget(posts.widget, body_chunks[1], &mut list_state);
                render_post_images(f, app, &posts.layouts, body_chunks[1], list_state.offset());
            }
            Tab::Notifications => {
                let body = draw::notifications(app.state());
                app.state
                    .get_notifications_list_state()
                    .select(Some(app.state.get_notifications_list_position()));
                f.render_stateful_widget(
                    body,
                    body_chunks[1],
                    &mut app.state.get_notifications_list_state(),
                );
            }
            Tab::Search => {
                let posts = draw::search_results(app.state(), body_chunks[1].width);
                let mut list_state = app.state.get_search_list_state();
                list_state.select(Some(app.state.get_search_list_position()));
                f.render_stateful_widget(posts.widget, body_chunks[1], &mut list_state);
                render_post_images(f, app, &posts.layouts, body_chunks[1], list_state.offset());
            }
        };
    }

    if app.state.is_help_mode() {
        let popup = draw::help();
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

    if let Some((url, alt, index, total)) = app.current_image_viewer() {
        render_image_viewer(f, app, &url, &alt, index, total, size);
    }

    if let Some((facets, selected)) = app.current_facet_viewer() {
        let popup = draw::facets(facets);
        let area = layout::popup(80, 60, size);
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut list_state);
    }

    if let Some((kind, items, selected)) = app.current_interactions() {
        let popup = draw::interactions(kind, items);
        let area = layout::popup(80, 70, size);
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut list_state);
    }

    if let Some(message) = app.error() {
        let popup = draw::error(message);
        let area = layout::popup(70, 30, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}

fn render_post_images(
    f: &mut Frame,
    app: &mut App,
    layouts: &[draw::PostLayout],
    area: ratatui::layout::Rect,
    offset: usize,
) {
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
    let block = Block::default().borders(Borders::ALL).title(format!(
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
