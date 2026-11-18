use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::game::{TileState, MAX_ROUNDS, WORD_LEN};

/// Width of one tile cell.
const TILE_WIDTH: u16 = 3;
/// Gap between the tiles of a row.
const TILE_GAP: u16 = 1;
/// Height of one tile cell.
const TILE_HEIGHT: u16 = 1;
/// Gap between two rows of tiles.
const ROW_GAP: u16 = 1;

/// Total width of the 6×5 board in columns.
const BOARD_WIDTH: u16 = WORD_LEN as u16 * TILE_WIDTH + (WORD_LEN as u16 - 1) * TILE_GAP;
/// Total height of the 6×5 board in rows.
const BOARD_HEIGHT: u16 = MAX_ROUNDS as u16 * TILE_HEIGHT + (MAX_ROUNDS as u16 - 1) * ROW_GAP;

/// Renders the application into the terminal frame.
///
/// The UI is a pure function of the app state: the transient message line on
/// top, the 6×5 board in the middle, and a quit hint at the bottom.
pub fn render(frame: &mut Frame, app: &App) {
    let [message_area, board_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let message = app.message().unwrap_or_default();
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD)),
        message_area,
    );

    render_board(frame, app, centered(board_area));

    frame.render_widget(
        Paragraph::new("q to quit")
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Renders the six rows of five tiles.
///
/// Played rounds show their letters with the evaluation's colors, the active
/// row shows the word being typed, and the remaining rows stay blank.
fn render_board(frame: &mut Frame, app: &App, grid: Rect) {
    let board = app.game().board();
    let played = app.game().rounds_played();
    let typed = app.typed();

    for row in 0..MAX_ROUNDS {
        for col in 0..WORD_LEN {
            let cell = tile_rect(grid, row, col);
            if cell.is_empty() {
                continue;
            }

            let (text, style) = match board.round(row) {
                Some(round) => {
                    let evaluation = round.evaluation();
                    (
                        display_letter(round.guess().as_str(), col),
                        tile_style(evaluation[col]),
                    )
                }
                None if row == played => (
                    display_letter(typed, col),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                None => (String::new(), Style::default()),
            };

            frame.render_widget(
                Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .style(style),
                cell,
            );
        }
    }
}

/// Style of a tile according to its evaluation.
fn tile_style(state: TileState) -> Style {
    let color = match state {
        TileState::Correct => Color::Green,
        TileState::Present => Color::Yellow,
        TileState::Absent => Color::DarkGray,
    };
    Style::default()
        .fg(Color::Black)
        .bg(color)
        .add_modifier(Modifier::BOLD)
}

/// The uppercase letter at `index` of `word`, if any.
fn display_letter(word: &str, index: usize) -> String {
    word.chars()
        .nth(index)
        .map(|letter| letter.to_ascii_uppercase().to_string())
        .unwrap_or_default()
}

/// The screen rectangle of one tile, clipped to the board area.
fn tile_rect(grid: Rect, row: usize, col: usize) -> Rect {
    Rect {
        x: grid.x + col as u16 * (TILE_WIDTH + TILE_GAP),
        y: grid.y + row as u16 * (TILE_HEIGHT + ROW_GAP),
        width: TILE_WIDTH,
        height: TILE_HEIGHT,
    }
    .intersection(grid)
}

/// The board rectangle centered in `area`, clamped to its size.
fn centered(area: Rect) -> Rect {
    let width = BOARD_WIDTH.min(area.width);
    let height = BOARD_HEIGHT.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
