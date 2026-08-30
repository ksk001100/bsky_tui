use ratatui::style::{Color, Modifier, Style};

// A small semantic palette keeps content readable across common dark terminal themes.
// User-selected accent colors are reserved for focus and navigation, not body copy.
pub const TEXT: Color = Color::White;
pub const MUTED: Color = Color::Gray;
pub const SUBTLE: Color = Color::DarkGray;
pub const POSITIVE: Color = Color::LightGreen;
pub const NEGATIVE: Color = Color::LightRed;

pub fn selected(accent: Color) -> Style {
    Style::default()
        .fg(contrast_text(accent))
        .bg(accent)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_content() -> Style {
    // A stable dark surface keeps author, metadata, and reaction colors legible together.
    Style::default()
        .fg(Color::White)
        .bg(Color::Rgb(30, 41, 59))
        .add_modifier(Modifier::BOLD)
}

pub fn accent(accent: Color) -> Style {
    Style::default().fg(accent).add_modifier(Modifier::BOLD)
}

pub fn border(accent: Color) -> Style {
    Style::default().fg(accent)
}

fn contrast_text(background: Color) -> Color {
    match background {
        Color::Black | Color::Blue | Color::Red | Color::Magenta => Color::White,
        _ => Color::Black,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_style_keeps_dark_accents_legible() {
        assert_eq!(selected(Color::Blue).fg, Some(Color::White));
        assert_eq!(selected(Color::Yellow).fg, Some(Color::Black));
    }
}
