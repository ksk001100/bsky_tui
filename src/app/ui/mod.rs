mod draw;
mod layout;

use ratatui::{backend::Backend, widgets::Clear, Frame};

use crate::app::{state::Tab, App};

pub fn render<B>(f: &mut Frame<B>, app: &App)
where
    B: Backend,
{
    let size = f.size();

    let main_chunks = layout::main(size);
    let header_chunks = layout::header(main_chunks[0]);
    let body_chunks = layout::body(main_chunks[1]);

    let title = draw::title();
    f.render_widget(title, header_chunks[0]);

    let mode = draw::mode(app.state());
    f.render_widget(mode, header_chunks[1]);

    let tabs = draw::tabs(app.state());
    f.render_widget(tabs, body_chunks[0]);

    match app.state.get_tab() {
        Tab::Timeline => {
            let body = draw::timeline(app.state());
            app.state
                .get_tl_list_state()
                .select(Some(app.state.get_tl_list_position()));
            f.render_stateful_widget(body, body_chunks[1], &mut app.state.get_tl_list_state());
            if app.state.get_feeds().is_none() {
                let popup = draw::loading();
                let area = layout::popup(60, 20, size);
                f.render_widget(Clear, area);
                f.render_widget(popup, area);
            }
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
            if app.state.get_notifications().is_none() {
                let popup = draw::loading();
                let area = layout::popup(60, 20, size);
                f.render_widget(Clear, area);
                f.render_widget(popup, area);
            }
        }
    };

    if app.state.is_help_mode() {
        let popup = draw::help();
        let area = layout::popup(60, 40, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }

    if app.state.is_post_mode() {
        let popup = draw::post_input(app.state());
        let area = layout::input_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        // f.set_cursor(
        //     area.x + 2 + app.state.get_input_cursor_position() as u16,
        //     area.y + 3,
        // );
    }

    if app.state.is_reply_mode() {
        let popup = draw::reply_input(app.state());
        let area = layout::reply_popup(size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        // f.set_cursor(
        //     area.x + 2 + app.state.get_input_cursor_position() as u16,
        //     area.y + 3,
        // );
    }
}
