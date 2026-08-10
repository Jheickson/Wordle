use std::io::Result;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::App;

/// Read the next terminal event and apply it to the app state.
///
/// Returns `Ok(true)` when the app should quit, `Ok(false)` otherwise.
///
/// TODO: handle letter keys (append to the current guess), Enter (submit),
/// Backspace (undo a letter), and win/loss state.
pub fn handle_events(_app: &mut App) -> Result<bool> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
            return Ok(true);
        }
    }
    Ok(false)
}
