mod draw;
mod layout;

use crate::app::App;
use ratatui::backend::Backend;
use ratatui::widgets::Clear;
use ratatui::Frame;

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

    let body = draw::body(app.state());
    app.state
        .get_tl_list_state()
        .select(Some(app.state.get_tl_list_position()));
    f.render_stateful_widget(body, body_chunks[0], &mut app.state.get_tl_list_state());

    if app.state.get_feed().is_none() {
        let popup = draw::loading();
        let area = layout::popup(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }

    if app.state.is_help_mode() {
        let popup = draw::help();
        let area = layout::popup(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }

    if app.state.is_post_mode() {
        let popup = draw::post_input(app.state());
        let area = layout::popup(60, 10, size);
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
        // f.set_cursor(
        //     area.x + 2 + app.state.get_input_cursor_position() as u16,
        //     area.y + 3,
        // );
    }
}
