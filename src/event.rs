use std::io::Result;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;
use crate::game::GameState;

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
    let finished = app.game().state() != GameState::Playing;

    match key.code {
        KeyCode::Char('q') => return true,
        // `n` and Enter replay only once the game ended; while playing they
        // keep their meaning of letter and submitted guess.
        KeyCode::Char('n') | KeyCode::Enter if finished => app.new_game(),
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

    use crate::game::{Answer, MAX_ROUNDS};

    fn app() -> App {
        App::with_answer(Answer::new("crane").expect("test answer is a five-letter word"))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_word(app: &mut App, word: &str) {
        for letter in word.chars() {
            handle_key(app, key(KeyCode::Char(letter)));
        }
    }

    fn win(app: &mut App) {
        type_word(app, "crane");
        handle_key(app, key(KeyCode::Enter));
        assert_eq!(app.game().state(), GameState::Won);
    }

    fn lose(app: &mut App) {
        for _ in 0..MAX_ROUNDS {
            type_word(app, "slate");
            handle_key(app, key(KeyCode::Enter));
        }
        assert_eq!(app.game().state(), GameState::Lost);
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
    fn n_is_a_letter_while_playing() {
        let mut app = app();

        handle_key(&mut app, key(KeyCode::Char('n')));

        assert_eq!(app.typed(), "n");
        assert_eq!(app.game().rounds_played(), 0);
    }

    #[test]
    fn n_starts_a_new_game_after_a_win() {
        let mut app = app();
        win(&mut app);

        handle_key(&mut app, key(KeyCode::Char('n')));

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn enter_starts_a_new_game_after_a_win() {
        let mut app = app();
        win(&mut app);

        handle_key(&mut app, key(KeyCode::Enter));

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
    }

    #[test]
    fn n_starts_a_new_game_after_a_loss() {
        let mut app = app();
        lose(&mut app);

        handle_key(&mut app, key(KeyCode::Char('n')));

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.message(), None);
    }

    #[test]
    fn enter_starts_a_new_game_after_a_loss() {
        let mut app = app();
        lose(&mut app);

        handle_key(&mut app, key(KeyCode::Enter));

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn unrelated_keys_are_ignored() {
        let mut app = app();

        assert!(!handle_key(&mut app, key(KeyCode::Esc)));
        assert_eq!(app.typed(), "");
    }
}
