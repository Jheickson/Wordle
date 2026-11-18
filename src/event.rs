use std::io::Result;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Screen};
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
    // Raw mode delivers Ctrl-C as a key event rather than SIGINT, so the
    // quit must be handled here. Ctrl-C quits from every screen; plain `c`
    // keeps its meaning of a typed letter below.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }
    // `q` quits on every screen except a game in progress, where it is a
    // typed letter like any other.

    match app.screen() {
        Screen::Menu => match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.select_previous_menu_entry(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next_menu_entry(),
            KeyCode::Char('q') => return true,
            KeyCode::Enter => return app.activate_menu_entry(),
            _ => {}
        },
        Screen::Game => {
            // `n` and Enter replay only once the game ended; while playing
            // they keep their meaning of letter and submitted guess.
            let finished = app.game().state() != GameState::Playing;
            match key.code {
                KeyCode::Esc => app.open_menu(),
                KeyCode::Char('n') | KeyCode::Enter if finished => app.new_game(),
                KeyCode::Char('q') if finished => return true,
                KeyCode::Char(letter) => app.input_letter(letter),
                KeyCode::Backspace => app.backspace(),
                KeyCode::Enter => app.submit(),
                _ => {}
            }
        }
        Screen::Credits | Screen::Repository => {
            match key.code {
                KeyCode::Char('q') => return true,
                KeyCode::Esc | KeyCode::Enter => app.open_menu(),
                _ => {}
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::MenuEntry;
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
    fn q_asks_to_quit_from_the_menu() {
        let mut app = app();
        app.open_menu();

        assert!(handle_key(&mut app, key(KeyCode::Char('q'))));
    }

    #[test]
    fn q_is_a_typed_letter_while_playing() {
        let mut app = app();

        assert!(!handle_key(&mut app, key(KeyCode::Char('q'))));
        assert_eq!(app.typed(), "q");
    }

    #[test]
    fn q_asks_to_quit_after_the_game_ended() {
        let mut app = app();
        win(&mut app);

        assert!(handle_key(&mut app, key(KeyCode::Char('q'))));
    }

    #[test]
    fn ctrl_c_asks_to_quit_while_playing() {
        let mut app = app();

        assert!(handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ));
        assert_eq!(app.typed(), "");
    }

    #[test]
    fn ctrl_c_asks_to_quit_after_the_game_ended() {
        let mut app = app();
        win(&mut app);

        assert!(handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ));
    }

    #[test]
    fn plain_c_is_a_typed_letter() {
        let mut app = app();

        assert!(!handle_key(&mut app, key(KeyCode::Char('c'))));
        assert_eq!(app.typed(), "c");
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

        assert!(!handle_key(&mut app, key(KeyCode::Tab)));
        assert_eq!(app.typed(), "");
    }

    #[test]
    fn esc_opens_the_menu_from_a_game() {
        let mut app = app();
        type_word(&mut app, "sla");

        assert!(!handle_key(&mut app, key(KeyCode::Esc)));
        assert_eq!(app.screen(), Screen::Menu);
        assert_eq!(app.typed(), "sla", "returning to the menu keeps the game");
    }

    #[test]
    fn arrow_keys_move_the_menu_selection() {
        let mut app = app();
        handle_key(&mut app, key(KeyCode::Esc));

        assert_eq!(app.menu_selection(), MenuEntry::StartGame);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.menu_selection(), MenuEntry::Credits);
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.menu_selection(), MenuEntry::StartGame);
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.menu_selection(), MenuEntry::Quit, "the cursor wraps up");
    }

    #[test]
    fn j_and_k_move_the_menu_selection() {
        let mut app = app();
        handle_key(&mut app, key(KeyCode::Esc));

        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.menu_selection(), MenuEntry::Credits);
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.menu_selection(), MenuEntry::StartGame);
    }

    #[test]
    fn enter_on_quit_quits() {
        let mut app = app();
        handle_key(&mut app, key(KeyCode::Esc));
        handle_key(&mut app, key(KeyCode::Up));

        assert!(handle_key(&mut app, key(KeyCode::Enter)));
    }

    #[test]
    fn enter_on_start_game_begins_a_fresh_game() {
        let mut app = app();
        type_word(&mut app, "slate");
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.game().rounds_played(), 1);

        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!handle_key(&mut app, key(KeyCode::Enter)));

        assert_eq!(app.screen(), Screen::Game);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn enter_opens_credits_and_esc_returns_to_the_menu() {
        let mut app = app();
        handle_key(&mut app, key(KeyCode::Esc));
        handle_key(&mut app, key(KeyCode::Down));

        assert!(!handle_key(&mut app, key(KeyCode::Enter)));
        assert_eq!(app.screen(), Screen::Credits);

        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen(), Screen::Menu);
    }

    #[test]
    fn enter_opens_the_repository_and_esc_returns_to_the_menu() {
        let mut app = app();
        handle_key(&mut app, key(KeyCode::Esc));
        handle_key(&mut app, key(KeyCode::Down));
        handle_key(&mut app, key(KeyCode::Down));

        assert!(!handle_key(&mut app, key(KeyCode::Enter)));
        assert_eq!(app.screen(), Screen::Repository);

        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen(), Screen::Menu);
    }

    #[test]
    fn letters_do_nothing_on_the_menu() {
        let mut app = app();
        type_word(&mut app, "sla");
        handle_key(&mut app, key(KeyCode::Esc));

        handle_key(&mut app, key(KeyCode::Char('t')));

        assert_eq!(app.typed(), "sla", "menu keys never reach the game");
        assert_eq!(app.game().rounds_played(), 0);
    }
}
