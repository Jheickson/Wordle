use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

/// Render the application into the terminal frame.
///
/// TODO: replace this placeholder with the Wordle grid (guesses with
/// green/yellow/grey feedback per letter).
pub fn render(frame: &mut Frame, _app: &App) {
    let text = Line::from("Hello, TUI World!");
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(paragraph, frame.area());
}
