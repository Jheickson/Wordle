use std::io::Result;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;

/// Reads the next terminal event and applies it to the app state.
///
/// Returns `Ok(true)` when the app should quit, `Ok(false)` otherwise.
pub fn handle_events(app: &mut App) -> Result<bool> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            return Ok(handle_key(app, key));
        }
    }
    Ok(false)
}

/// Translates one key press into an [`App`] action.
///
/// Returns `true` when the key means "quit".
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char(letter) => app.input_letter(letter),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Enter => app.submit(),
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    use crate::game::Answer;

    fn app() -> App {
        App::with_answer(Answer::new("crane").expect("test answer is a five-letter word"))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn letters_are_forwarded_to_the_app() {
        let mut app = app();

        assert!(!handle_key(&mut app, key(KeyCode::Char('s'))));
        assert!(!handle_key(&mut app, key(KeyCode::Char('L'))));
        assert!(!handle_key(&mut app, key(KeyCode::Char('a'))));

        assert_eq!(app.typed(), "sla");
    }

    #[test]
    fn backspace_is_forwarded_to_the_app() {
        let mut app = app();

        handle_key(&mut app, key(KeyCode::Char('s')));
        handle_key(&mut app, key(KeyCode::Backspace));

        assert_eq!(app.typed(), "");
    }

    #[test]
    fn enter_submits_the_typed_word() {
        let mut app = app();

        handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.message().is_some());
        assert_eq!(app.game().rounds_played(), 0);
    }

    #[test]
    fn q_asks_to_quit() {
        let mut app = app();

        assert!(handle_key(&mut app, key(KeyCode::Char('q'))));
    }

    #[test]
    fn unrelated_keys_are_ignored() {
        let mut app = app();

        assert!(!handle_key(&mut app, key(KeyCode::Esc)));
        assert_eq!(app.typed(), "");
    }
}
