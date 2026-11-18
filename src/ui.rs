use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, MenuEntry, Screen};
use crate::game::{keyboard_index, GameState, TileState};

/// Width of one tile cell.
const TILE_WIDTH: u16 = 3;
/// Gap between the tiles of a row.
const TILE_GAP: u16 = 1;
/// Height of one tile cell.
const TILE_HEIGHT: u16 = 1;
/// Gap between two rows of tiles.
const ROW_GAP: u16 = 1;

/// Total width in columns of a board whose words have `word_len` letters.
fn board_width(word_len: usize) -> u16 {
    word_len as u16 * TILE_WIDTH + (word_len as u16 - 1) * TILE_GAP
}

/// Total height in rows of a board with `rounds` rows of tiles.
fn board_height(rounds: usize, row_gap: u16) -> u16 {
    rounds as u16 * TILE_HEIGHT + (rounds as u16 - 1) * row_gap
}

/// Rows of the on-screen keyboard, top to bottom.
const KEYBOARD_ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];
/// Height of the keyboard in rows.
const KEYBOARD_HEIGHT: u16 = KEYBOARD_ROWS.len() as u16;

/// Width of the menu block in columns.
const MENU_WIDTH: u16 = 36;

/// Key hint shown on the menu screen.
const MENU_HINT: &str = "↑/↓ select · Enter confirm · q quit";

/// Key hint shown while the Start game entry is hovered, where ←/→ pick the
/// word size and -/+ pick the guess limit.
const START_GAME_HINT: &str =
    "↑/↓ select · ←/→ letters · -/+ guesses · Enter confirm · q quit";

/// Text of the credits screen.
const CREDITS_TEXT: &str = "WORDLE\n\nA terminal Wordle clone, built with\nRust, ratatui, and crossterm.";

/// Text of the repository screen.
const REPOSITORY_TEXT: &str = "Repository\n\nhttps://github.com/Jheickson/Wordle";

/// Renders the application into the terminal frame.
///
/// The UI is a pure function of the app state: the screen being shown picks
/// the layout among the menu, the credits, the repository, and the game.
pub fn render(frame: &mut Frame, app: &App) {
    match app.screen() {
        Screen::Menu => render_menu(frame, app, frame.area()),
        Screen::Credits => render_text_screen(frame, CREDITS_TEXT, "Esc: back", frame.area()),
        Screen::Repository => {
            render_text_screen(frame, REPOSITORY_TEXT, "Esc: back", frame.area())
        }
        Screen::Game => render_game(frame, app, frame.area()),
    }
}

/// Renders the main menu: title, its selectable entries, and a key hint.
///
/// The entry under the cursor is drawn reversed to set it apart from the
/// others.
fn render_menu(frame: &mut Frame, app: &App, area: Rect) {
    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let mut lines = vec![
        Line::from(Span::styled(
            "WORDLE",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::default(),
    ];
    lines.extend(MenuEntry::ALL.iter().map(|&entry| {
        let style = if entry == app.menu_selection() {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        // The Start game entry shows the word size picked with ←/→ and the
        // guess limit picked with -/+.
        let title = if entry == MenuEntry::StartGame {
            format!(
                "Start game ({} letters, {} guesses)",
                app.word_size(),
                app.max_rounds()
            )
        } else {
            entry.title().to_string()
        };
        Line::from(Span::styled(title, style))
    }));
    let height = lines.len() as u16;

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered_within(content_area, MENU_WIDTH, height),
    );

    let hint = if app.menu_selection() == MenuEntry::StartGame {
        START_GAME_HINT
    } else {
        MENU_HINT
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Renders a centered block of text with a hint at the bottom.
fn render_text_screen(frame: &mut Frame, text: &str, hint: &str, area: Rect) {
    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let lines: Vec<Line> = text.lines().map(Line::from).collect();
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16;
    let height = lines.len() as u16;

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered_within(content_area, width, height),
    );

    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Renders the game screen: message line, board, keyboard, and key hint.
fn render_game(frame: &mut Frame, app: &App, area: Rect) {
    let [message_area, board_area, keyboard_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(KEYBOARD_HEIGHT),
        Constraint::Length(1),
    ])
    .areas(area);

    let message = app.message().unwrap_or_default();
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD)),
        message_area,
    );

    let rounds = app.game().max_rounds();
    // A tall board (up to 15 rows) may only fit without the row gaps.
    let row_gap = if board_height(rounds, ROW_GAP) <= board_area.height {
        ROW_GAP
    } else {
        0
    };
    render_board(
        frame,
        app,
        centered_within(
            board_area,
            board_width(app.game().word_len()),
            board_height(rounds, row_gap),
        ),
        row_gap,
    );
    render_keyboard(frame, app, keyboard_area);

    // While a game is playing, `q` is a typed letter; it only quits once the
    // game has ended.
    let hint = match app.game().state() {
        GameState::Playing => "Esc: menu",
        GameState::Won | GameState::Lost => "Esc: menu · q to quit",
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Renders the board: one row per round the game allows, one tile per letter.
///
/// Played rounds show their letters with the evaluation's colors, the active
/// row shows the word being typed, and every cell without a guess yet shows a
/// dimmed placeholder square so the player can see how many guesses remain.
fn render_board(frame: &mut Frame, app: &App, grid: Rect, row_gap: u16) {
    let board = app.game().board();
    let played = app.game().rounds_played();
    let typed = app.typed();

    for row in 0..app.game().max_rounds() {
        for col in 0..app.game().word_len() {
            let cell = tile_rect(grid, row, col, row_gap);
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
                None if row == played => {
                    let letter = display_letter(typed, col);
                    if letter.is_empty() {
                        (String::new(), placeholder_style())
                    } else {
                        (
                            letter,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        )
                    }
                }
                None => (String::new(), placeholder_style()),
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

/// Renders the QWERTY keyboard below the board.
///
/// Each key is colored with the best tile state seen for its letter; keys
/// that were never guessed keep the plain style.
fn render_keyboard(frame: &mut Frame, app: &App, area: Rect) {
    let states = app.game().keyboard_states();

    for (row_index, row) in KEYBOARD_ROWS.iter().enumerate() {
        let mut spans = Vec::with_capacity(row.len() * 2);
        for (index, letter) in row.chars().enumerate() {
            if index > 0 {
                spans.push(Span::raw(" "));
            }
            let style = match states[keyboard_index(letter)] {
                Some(state) => tile_style(state),
                None => Style::default().fg(Color::White),
            };
            spans.push(Span::styled(format!(" {letter} "), style));
        }

        let width = row.len() as u16 * (TILE_WIDTH + TILE_GAP) - TILE_GAP;
        let rect = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + row_index as u16,
            width: width.min(area.width),
            height: 1,
        }
        .intersection(area);

        frame.render_widget(Paragraph::new(Line::from(spans)), rect);
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

/// Style of a tile with no guess yet: a dimmed square that shows the player
/// how many guesses remain.
fn placeholder_style() -> Style {
    Style::default().bg(Color::DarkGray)
}

/// The uppercase letter at `index` of `word`, if any.
fn display_letter(word: &str, index: usize) -> String {
    word.chars()
        .nth(index)
        .map(|letter| letter.to_ascii_uppercase().to_string())
        .unwrap_or_default()
}

/// The screen rectangle of one tile, clipped to the board area.
fn tile_rect(grid: Rect, row: usize, col: usize, row_gap: u16) -> Rect {
    Rect {
        x: grid.x + col as u16 * (TILE_WIDTH + TILE_GAP),
        y: grid.y + row as u16 * (TILE_HEIGHT + row_gap),
        width: TILE_WIDTH,
        height: TILE_HEIGHT,
    }
    .intersection(grid)
}

/// The rectangle of `width`×`height` centered in `area`, clamped to its size.
fn centered_within(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
