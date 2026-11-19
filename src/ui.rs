use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, MenuEntry, Screen};
use crate::game::{keyboard_index, GameState, TileState};

/// Smallest tile size, in columns×rows: the layout floor on cramped terminals.
const MIN_TILE_WIDTH: u16 = 3;
const MIN_TILE_HEIGHT: u16 = 1;
/// Largest tile size, in columns×rows: the layout stops growing here so huge
/// terminals keep a compact grid instead of a wall of blocks.
const MAX_TILE_WIDTH: u16 = 7;
const MAX_TILE_HEIGHT: u16 = 3;
/// Gap between the tiles of one board row.
const TILE_GAP: u16 = 1;
/// Gap between the keys of one keyboard row.
const KEY_GAP: u16 = 1;

/// Rows of the on-screen keyboard, top to bottom.
const KEYBOARD_ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

/// Sizes of the game screen components, derived from the viewport at render
/// time so the screen fills the space it is given.
#[derive(Debug, Clone, Copy)]
struct GameMetrics {
    /// Columns and rows of one tile.
    tile_width: u16,
    tile_height: u16,
    /// Rows of gap between two board rows; drops to zero when the board does
    /// not fit with it.
    row_gap: u16,
    /// Columns and rows of one keyboard key.
    key_width: u16,
    key_height: u16,
}

impl GameMetrics {
    /// Total width in columns of a board whose words have `word_len` letters.
    fn board_width(&self, word_len: usize) -> u16 {
        word_len as u16 * self.tile_width + (word_len as u16 - 1) * TILE_GAP
    }

    /// Total height in rows of a board with `rounds` rows of tiles.
    fn board_height(&self, rounds: usize) -> u16 {
        rounds as u16 * self.tile_height + (rounds as u16 - 1) * self.row_gap
    }

    /// Total height in rows of the on-screen keyboard.
    fn keyboard_height(&self) -> u16 {
        KEYBOARD_ROWS.len() as u16 * self.key_height
    }
}

/// Computes the game screen component sizes for a viewport of `area` holding
/// a board of `rounds` rows with words of `word_len` letters.
///
/// Tiles come in the steps 3×1, 5×2, and 7×3: a tile is twice as wide as it
/// is tall, plus one column, which keeps it visually square on terminal
/// cells. The step is the largest that fits both the height left once the
/// message, the key hint, and a minimum-size keyboard are reserved, and the
/// width of the board. Height the capped board leaves over goes to taller
/// keyboard keys, which pair their size with the tiles the same way.
fn game_metrics(area: Rect, rounds: usize, word_len: usize) -> GameMetrics {
    let keyboard_rows = KEYBOARD_ROWS.len() as u16;
    let board_height_avail = area.height.saturating_sub(2 + keyboard_rows);
    let board_width_avail = area.width.saturating_sub(word_len as u16 - 1);

    // Every step of tile height costs `rounds` rows plus one gap row.
    let height_step = board_height_avail.saturating_sub(rounds as u16 - 1) / rounds as u16;
    // Every step of tile width costs `word_len` columns plus one gap column,
    // and two columns plus one make one row of tile height.
    let width_step = board_width_avail.saturating_sub(word_len as u16) / (2 * word_len as u16);
    let tile_height = height_step
        .min(width_step)
        .clamp(MIN_TILE_HEIGHT, MAX_TILE_HEIGHT);
    let tile_width = 2 * tile_height + 1;

    // The board keeps its one-row gaps whenever it fits with them.
    let board_height = rounds as u16 * tile_height;
    let row_gap = u16::from(board_height + (rounds as u16 - 1) <= board_height_avail);

    let leftover = board_height_avail.saturating_sub(board_height + (rounds as u16 - 1) * row_gap);
    let key_height = (1 + leftover / keyboard_rows).clamp(MIN_TILE_HEIGHT, MAX_TILE_HEIGHT);
    // The top keyboard row is the widest, so it bounds the key width.
    let top_keys = KEYBOARD_ROWS[0].len() as u16;
    let key_width = (2 * key_height + 1)
        .min(area.width.saturating_sub(top_keys - 1) / top_keys)
        .clamp(MIN_TILE_WIDTH, MAX_TILE_WIDTH);

    GameMetrics {
        tile_width,
        tile_height,
        row_gap,
        key_width,
        key_height,
    }
}

/// Key hint shown on the menu screen.
const MENU_HINT: &str = "↑/↓ select · Enter confirm · q quit";

/// Key hint shown while the Start game entry is hovered, where ←/→ pick the
/// word size and -/+ pick the guess limit.
const START_GAME_HINT: &str = "↑/↓ select · ←/→ letters · -/+ guesses · Enter confirm · q quit";

/// Text of the credits screen.
const CREDITS_TEXT: &str =
    "WORDLE\n\nA terminal Wordle clone, built with\nRust, ratatui, and crossterm.";

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
        Screen::Repository => render_text_screen(frame, REPOSITORY_TEXT, "Esc: back", frame.area()),
        Screen::Game => render_game(frame, app, frame.area()),
    }
}

/// Block-letter glyphs for the menu title, one string per row, five rows per
/// letter.
const TITLE_LETTERS: [[&str; 5]; 6] = [
    ["#   #", "#   #", "# # #", "# # #", " # # "], // W
    [" ### ", "#   #", "#   #", "#   #", " ### "], // O
    ["#### ", "#   #", "#### ", "#  # ", "#   #"], // R
    ["#### ", "#   #", "#   #", "#   #", "#### "], // D
    ["#    ", "#    ", "#    ", "#    ", "#####"], // L
    ["#####", "#    ", "#### ", "#    ", "#####"], // E
];
/// Blank columns between two title letters, per unit of title scale.
const TITLE_GAP: u16 = 1;

/// Width in columns of the block-letter title at `scale`.
fn title_glyph_width(scale: u16) -> u16 {
    let letters: u16 = TITLE_LETTERS
        .iter()
        .map(|glyph| glyph[0].len() as u16)
        .sum();
    (letters + TITLE_GAP * (TITLE_LETTERS.len() as u16 - 1)) * scale
}

/// Title scale for the menu: 0 falls back to a plain one-line title, 1 shows
/// the block letters, 2 stretches every glyph cell to a 2×2 block of cells. A
/// scale only applies when the screen has room for it.
fn menu_title_scale(area: Rect) -> u16 {
    if area.height >= 34 && area.width >= title_glyph_width(2) + 2 {
        2
    } else if area.height >= 16 && area.width >= title_glyph_width(1) + 1 {
        1
    } else {
        0
    }
}

/// The menu title rendered as block letters at `scale`, every row padded to
/// `width` columns so it centers inside the menu block.
fn title_lines(scale: u16, width: u16) -> Vec<Line<'static>> {
    let scale = scale as usize;
    let style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut lines = Vec::with_capacity(5 * scale);
    for glyph_row in 0..5 {
        let mut row = String::new();
        for (index, glyph) in TITLE_LETTERS.iter().enumerate() {
            if index > 0 {
                row.push_str(&" ".repeat(TITLE_GAP as usize * scale));
            }
            for cell in glyph[glyph_row].chars() {
                for _ in 0..scale {
                    row.push(cell);
                }
            }
        }
        let line = Line::from(Span::styled(pad_centered(&row, width), style));
        for _ in 0..scale {
            lines.push(line.clone());
        }
    }
    lines
}

/// Width of a menu entry selection bar: two fifths of the screen, at least
/// wide enough for the longest entry title with padding, and never wider
/// than the menu block.
fn menu_bar_width(area: Rect) -> u16 {
    (area.width * 2 / 5).clamp(40, 72).min(area.width)
}

/// Renders the main menu: block-letter title (plain text while the screen is
/// too small for the letters), the selectable entries as full-width bars
/// with the chosen one reversed, and a key hint.
fn render_menu(frame: &mut Frame, app: &App, area: Rect) {
    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let scale = menu_title_scale(content_area);
    let bar_width = menu_bar_width(content_area);
    let block_width = bar_width.max(if scale > 0 {
        title_glyph_width(scale)
    } else {
        6
    });

    let mut lines = Vec::new();
    if scale == 0 {
        lines.push(Line::from(Span::styled(
            pad_centered("WORDLE", block_width),
            Style::default().add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.extend(title_lines(scale, block_width));
    }
    lines.push(Line::default());

    // Blank gaps between the entries soak up spare height; the entry under
    // the cursor is reversed across the whole bar so it reads as a button.
    let entries = MenuEntry::ALL.len() as u16;
    let used = lines.len() as u16 + entries;
    let entry_gap = (content_area.height.saturating_sub(used) / (entries - 1)).min(2);
    for (index, &entry) in MenuEntry::ALL.iter().enumerate() {
        if index > 0 {
            for _ in 0..entry_gap {
                lines.push(Line::default());
            }
        }
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
        let style = if entry == app.menu_selection() {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            pad_centered(&title, bar_width),
            style,
        )));
    }

    let height = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines),
        centered_within(content_area, block_width, height),
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
///
/// All component sizes come from [`game_metrics`], so the screen fills the
/// terminal instead of sitting in a fixed-size frame.
fn render_game(frame: &mut Frame, app: &App, area: Rect) {
    let rounds = app.game().max_rounds();
    let word_len = app.game().word_len();
    let metrics = game_metrics(area, rounds, word_len);

    let [message_area, board_area, keyboard_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(metrics.keyboard_height()),
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

    let grid = centered_within(
        board_area,
        metrics.board_width(word_len),
        metrics.board_height(rounds),
    );
    render_board(frame, app, grid, &metrics);
    render_keyboard(frame, app, keyboard_area, &metrics);

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
/// Tiles may be several rows tall; letters land on the middle row and the
/// tile's background fills the rest.
fn render_board(frame: &mut Frame, app: &App, grid: Rect, metrics: &GameMetrics) {
    let board = app.game().board();
    let played = app.game().rounds_played();
    let typed = app.typed();

    for row in 0..app.game().max_rounds() {
        for col in 0..app.game().word_len() {
            let cell = tile_rect(grid, row, col, metrics);
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
                Paragraph::new(center_vertically(&text, metrics.tile_height))
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
/// that were never guessed keep the plain style. Keys may be several rows
/// tall; letters land on the middle row, and colored keys fill their whole
/// rectangle.
fn render_keyboard(frame: &mut Frame, app: &App, area: Rect, metrics: &GameMetrics) {
    let states = app.game().keyboard_states();

    for (row_index, row) in KEYBOARD_ROWS.iter().enumerate() {
        let keys = row.chars().count() as u16;
        let row_width = keys * metrics.key_width + (keys - 1) * KEY_GAP;
        let row_area = Rect {
            x: area.x + area.width.saturating_sub(row_width) / 2,
            y: area.y + row_index as u16 * metrics.key_height,
            width: row_width.min(area.width),
            height: metrics.key_height,
        }
        .intersection(area);

        for (index, letter) in row.chars().enumerate() {
            let key = Rect {
                x: row_area.x + index as u16 * (metrics.key_width + KEY_GAP),
                y: row_area.y,
                width: metrics.key_width,
                height: metrics.key_height,
            }
            .intersection(row_area);

            let style = match states[keyboard_index(letter)] {
                Some(state) => tile_style(state),
                None => Style::default().fg(Color::White),
            };
            frame.render_widget(
                Paragraph::new(center_vertically(&letter.to_string(), metrics.key_height))
                    .alignment(Alignment::Center)
                    .style(style),
                key,
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
fn tile_rect(grid: Rect, row: usize, col: usize, metrics: &GameMetrics) -> Rect {
    Rect {
        x: grid.x + col as u16 * (metrics.tile_width + TILE_GAP),
        y: grid.y + row as u16 * (metrics.tile_height + metrics.row_gap),
        width: metrics.tile_width,
        height: metrics.tile_height,
    }
    .intersection(grid)
}

/// The text padded with blank lines so it lands on the middle row of a cell
/// `height` rows tall; the cell's background style fills every row either
/// way.
fn center_vertically(text: &str, height: u16) -> String {
    let above = (height - 1) / 2;
    let below = height - 1 - above;
    let mut padded = String::new();
    for _ in 0..above {
        padded.push('\n');
    }
    padded.push_str(text);
    for _ in 0..below {
        padded.push('\n');
    }
    padded
}

/// The text centered in a string of exactly `width` columns.
fn pad_centered(text: &str, width: u16) -> String {
    let text_width = text.chars().count() as u16;
    let width = width.max(text_width);
    let left = (width - text_width) / 2;
    let right = width - text_width - left;
    let mut padded = " ".repeat(left as usize);
    padded.push_str(text);
    padded.push_str(&" ".repeat(right as usize));
    padded
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
