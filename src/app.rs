//! Application state (the model).
//!
//! [`App`] owns the [`Game`], the word the player is currently typing, and a
//! transient message line. All input semantics live here as plain methods so
//! they can be unit-tested without a terminal; the UI only renders the
//! resulting state and the event loop only translates keys into these calls.

use crate::game::{Answer, Game, GameError, GameState, WORD_LEN};
use crate::words;

/// Message shown when the submitted word is too short.
const TOO_SHORT_MESSAGE: &str = "Not enough letters";

/// Message shown when the submitted word is not in the allowed list.
const UNKNOWN_WORD_MESSAGE: &str = "Not in word list";

/// Message shown after the winning guess.
const WON_MESSAGE: &str = "You won!";

/// Message shown when the board filled up without a win.
const LOST_MESSAGE: &str = "Game over";

/// Application state (the model).
#[derive(Debug)]
pub struct App {
    game: Game,
    typed: String,
    message: Option<String>,
}

impl App {
    /// Starts a new game with a randomly drawn answer.
    pub fn new() -> Self {
        Self::with_answer(words::random_answer())
    }

    /// Starts a new game with a fixed answer, for tests and deterministic
    /// setups.
    pub fn with_answer(answer: Answer) -> Self {
        Self {
            game: Game::new(answer),
            typed: String::new(),
            message: None,
        }
    }

    /// The game being played.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The word currently being typed.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// The transient message line, if there is one.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Appends a letter to the word being typed.
    ///
    /// Non-alphabetic keys and overflow beyond [`WORD_LEN`] are ignored. Input
    /// only acts while the game is still being played, so a finished game keeps
    /// its end-state message on screen. Typing clears any stale message.
    pub fn input_letter(&mut self, letter: char) {
        if self.game.state() != GameState::Playing
            || !letter.is_ascii_alphabetic()
            || self.typed.len() >= WORD_LEN
        {
            return;
        }
        self.typed.push(letter.to_ascii_lowercase());
        self.message = None;
    }

    /// Removes the last letter of the word being typed.
    ///
    /// A no-op when the word is empty or the game has ended.
    pub fn backspace(&mut self) {
        if self.game.state() != GameState::Playing {
            return;
        }
        if self.typed.pop().is_some() {
            self.message = None;
        }
    }

    /// Submits the word being typed.
    ///
    /// Rejects short and unknown words with a transient message without
    /// touching the game. A valid word is recorded on the board and the typed
    /// buffer cleared; a winning or losing submission leaves a visible
    /// end-state message, while an ongoing game stays silent. Submitting after
    /// the game ended is a no-op.
    pub fn submit(&mut self) {
        if self.game.state() != GameState::Playing {
            return;
        }

        if self.typed.len() < WORD_LEN {
            self.message = Some(TOO_SHORT_MESSAGE.to_string());
            return;
        }

        if !words::is_valid_guess(&self.typed) {
            self.message = Some(UNKNOWN_WORD_MESSAGE.to_string());
            return;
        }

        // Move the word out so the game can borrow it while `self` stays usable
        // for the follow-up state change.
        let word = std::mem::take(&mut self.typed);
        let result = self.game.submit_guess(&word).map(|_| ());

        match result {
            Ok(()) => {
                self.message = match self.game.state() {
                    GameState::Playing => None,
                    GameState::Won => Some(WON_MESSAGE.to_string()),
                    GameState::Lost => Some(LOST_MESSAGE.to_string()),
                };
            }
            // Unreachable: the checks above guarantee a playing game and a
            // five-letter valid word. Restore the buffer so no input is lost.
            Err(GameError::GameOver) => {
                self.typed = word;
                self.message = Some(LOST_MESSAGE.to_string());
            }
            Err(GameError::InvalidGuess(_)) => {
                self.typed = word;
                self.message = Some(UNKNOWN_WORD_MESSAGE.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MAX_ROUNDS;

    fn app_with(answer: &str) -> App {
        App::with_answer(Answer::new(answer).expect("test answer is a five-letter word"))
    }

    fn type_word(app: &mut App, word: &str) {
        for letter in word.chars() {
            app.input_letter(letter);
        }
    }

    #[test]
    fn input_letters_fill_the_typed_word_and_ignore_overflow() {
        let mut app = app_with("crane");

        type_word(&mut app, "slate");
        assert_eq!(app.typed(), "slate");

        app.input_letter('x');
        assert_eq!(app.typed(), "slate");
    }

    #[test]
    fn input_letters_ignore_non_letters() {
        let mut app = app_with("crane");

        app.input_letter('1');
        app.input_letter(' ');
        app.input_letter('é');
        assert_eq!(app.typed(), "");

        app.input_letter('S');
        assert_eq!(app.typed(), "s");
    }

    #[test]
    fn backspace_removes_the_last_letter() {
        let mut app = app_with("crane");

        type_word(&mut app, "sl");
        app.backspace();

        assert_eq!(app.typed(), "s");
    }

    #[test]
    fn backspace_on_empty_word_is_a_no_op() {
        let mut app = app_with("crane");

        app.backspace();

        assert_eq!(app.typed(), "");
        assert_eq!(app.game().rounds_played(), 0);
    }

    #[test]
    fn unknown_word_sets_message_and_records_no_round() {
        let mut app = app_with("crane");

        type_word(&mut app, "zzzzz");
        app.submit();

        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.message(), Some(UNKNOWN_WORD_MESSAGE));
        assert_eq!(app.typed(), "zzzzz", "the player can fix the word");
    }

    #[test]
    fn short_word_sets_message_and_records_no_round() {
        let mut app = app_with("crane");

        type_word(&mut app, "sl");
        app.submit();

        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.message(), Some(TOO_SHORT_MESSAGE));
        assert_eq!(app.typed(), "sl");
    }

    #[test]
    fn typing_clears_a_stale_message() {
        let mut app = app_with("crane");

        type_word(&mut app, "zzzzz");
        app.submit();
        assert!(app.message().is_some());

        app.backspace();
        assert_eq!(app.message(), None);
    }

    #[test]
    fn valid_word_advances_the_board() {
        let mut app = app_with("crane");

        type_word(&mut app, "slate");
        app.submit();

        assert_eq!(app.game().rounds_played(), 1);
        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
        assert_eq!(
            app.game().board().round(0).unwrap().guess().as_str(),
            "slate"
        );
    }

    #[test]
    fn winning_word_wins_the_game_and_shows_a_message() {
        let mut app = app_with("crane");

        type_word(&mut app, "crane");
        app.submit();

        assert_eq!(app.game().state(), GameState::Won);
        assert_eq!(app.game().rounds_played(), 1);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), Some(WON_MESSAGE));
    }

    #[test]
    fn six_wrong_guesses_lose_the_game() {
        let mut app = app_with("crane");

        for _ in 0..MAX_ROUNDS {
            type_word(&mut app, "slate");
            app.submit();
            assert_eq!(app.typed(), "");
        }

        assert_eq!(app.game().state(), GameState::Lost);
        assert_eq!(app.game().rounds_played(), MAX_ROUNDS);
        assert_eq!(app.message(), Some(LOST_MESSAGE));
    }

    #[test]
    fn input_is_ignored_after_the_game_ends() {
        let mut app = app_with("crane");

        type_word(&mut app, "crane");
        app.submit();

        app.input_letter('s');
        app.backspace();
        app.submit();

        assert_eq!(app.typed(), "");
        assert_eq!(app.game().rounds_played(), 1);
        assert_eq!(app.message(), Some(WON_MESSAGE));
    }
}
