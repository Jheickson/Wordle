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

/// Opening of the message shown when the board filled up without a win; the
/// revealed answer follows it.
const LOST_MESSAGE_PREFIX: &str = "The word was";

/// Invitation to start another game, appended to both end-state messages.
const NEW_GAME_PROMPT: &str = "press n for a new game";

/// The full winning message, including the next-game prompt.
fn won_message() -> String {
    format!("{WON_MESSAGE} {NEW_GAME_PROMPT}")
}
/// One selectable entry of the main menu, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEntry {
    /// Starts a fresh game.
    StartGame,
    /// Shows who built the game.
    Credits,
    /// Shows where the source lives.
    Repository,
    /// Exits the app.
    Quit,
}

impl MenuEntry {
    /// Every entry in display order.
    pub const ALL: [MenuEntry; 4] = [
        MenuEntry::StartGame,
        MenuEntry::Credits,
        MenuEntry::Repository,
        MenuEntry::Quit,
    ];

    /// The label shown on screen.
    pub fn title(self) -> &'static str {
        match self {
            MenuEntry::StartGame => "Start game",
            MenuEntry::Credits => "Credits",
            MenuEntry::Repository => "Repository",
            MenuEntry::Quit => "Quit",
        }
    }
}

/// Which screen the app is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The selectable main menu, shown on startup.
    Menu,
    /// An in-progress game.
    Game,
    /// The credits text.
    Credits,
    /// The repository link.
    Repository,
}

/// Application state (the model).
#[derive(Debug)]
pub struct App {
    game: Game,
    typed: String,
    message: Option<String>,
    screen: Screen,
    /// Index into [`MenuEntry::ALL`]; only mutated by the select methods.
    menu_selection: usize,
}

impl App {
    /// Starts a new game with a randomly drawn answer and opens on the menu.
    pub fn new() -> Self {
        let mut app = Self::with_answer(words::random_answer());
        app.screen = Screen::Menu;
        app
    }

    /// Starts a new game with a fixed answer, opening straight onto the game
    /// screen, for tests and deterministic setups.
    pub fn with_answer(answer: Answer) -> Self {
        Self {
            game: Game::new(answer),
            typed: String::new(),
            message: None,
            screen: Screen::Game,
            menu_selection: 0,
        }
    }

    /// The game being played.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Starts a fresh game with a newly drawn answer.
    ///
    /// The board, the typed word, and the message line are reset, so the app
    /// looks exactly as it does right after startup.
    pub fn new_game(&mut self) {
        self.game = Game::new(words::random_answer());
        self.typed.clear();
        self.message = None;
    }

    /// The screen currently shown.
    pub fn screen(&self) -> Screen {
        self.screen
    }

    /// The menu entry the cursor rests on.
    pub fn menu_selection(&self) -> MenuEntry {
        MenuEntry::ALL[self.menu_selection]
    }

    /// Moves the menu cursor up, wrapping from the first entry to the last.
    pub fn select_previous_menu_entry(&mut self) {
        self.menu_selection =
            (self.menu_selection + MenuEntry::ALL.len() - 1) % MenuEntry::ALL.len();
    }

    /// Moves the menu cursor down, wrapping from the last entry to the first.
    pub fn select_next_menu_entry(&mut self) {
        self.menu_selection = (self.menu_selection + 1) % MenuEntry::ALL.len();
    }

    /// Runs the menu entry under the cursor; returns `true` when the app
    /// should quit.
    ///
    /// Start game resets the board, the typed word, and the message line,
    /// exactly like the `n` replay after a finished game.
    pub fn activate_menu_entry(&mut self) -> bool {
        match self.menu_selection() {
            MenuEntry::StartGame => {
                self.new_game();
                self.screen = Screen::Game;
            }
            MenuEntry::Credits => self.screen = Screen::Credits,
            MenuEntry::Repository => self.screen = Screen::Repository,
            MenuEntry::Quit => return true,
        }
        false
    }

    /// Returns from any screen to the menu.
    ///
    /// The game underneath is left untouched; starting a game from the menu
    /// always begins a fresh one via [`App::activate_menu_entry`].
    pub fn open_menu(&mut self) {
        self.screen = Screen::Menu;
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
                    GameState::Won => Some(won_message()),
                    GameState::Lost => Some(self.lost_message()),
                };
            }
            // Unreachable: the checks above guarantee a playing game and a
            // five-letter valid word. Restore the buffer so no input is lost.
            Err(GameError::GameOver) => {
                self.typed = word;
                self.message = Some(self.lost_message());
            }
            Err(GameError::InvalidGuess(_)) => {
                self.typed = word;
                self.message = Some(UNKNOWN_WORD_MESSAGE.to_string());
            }
        }
    }

    /// The message shown once the final guess failed, revealing the answer and
    /// inviting another game.
    fn lost_message(&self) -> String {
        format!(
            "{LOST_MESSAGE_PREFIX} {}; {NEW_GAME_PROMPT}",
            self.game.answer().as_str().to_ascii_uppercase()
        )
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
        assert_eq!(app.message(), Some(won_message().as_str()));
        assert!(app.message().unwrap().contains(NEW_GAME_PROMPT));
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
    }

    #[test]
    fn losing_message_reveals_the_answer() {
        let mut app = app_with("swamp");

        for _ in 0..MAX_ROUNDS {
            type_word(&mut app, "slate");
            app.submit();
        }

        let message = app.message().expect("a loss leaves a message");
        assert!(message.contains("SWAMP"), "message was {message:?}");
        assert!(message.contains(NEW_GAME_PROMPT), "message was {message:?}");
    }

    #[test]
    fn new_game_after_a_win_starts_fresh() {
        let mut app = app_with("crane");
        type_word(&mut app, "crane");
        app.submit();
        assert_eq!(app.game().state(), GameState::Won);

        app.new_game();

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn new_game_after_a_loss_starts_fresh() {
        let mut app = app_with("crane");
        for _ in 0..MAX_ROUNDS {
            type_word(&mut app, "slate");
            app.submit();
        }
        assert_eq!(app.game().state(), GameState::Lost);

        app.new_game();

        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn new_game_draws_a_valid_playable_answer() {
        let mut app = app_with("crane");
        type_word(&mut app, "crane");
        app.submit();
        app.new_game();

        let answer = app.game().answer().as_str().to_string();
        assert!(
            words::is_valid_guess(&answer),
            "drawn answer {answer:?} is not a valid guess"
        );

        type_word(&mut app, &answer);
        app.submit();

        assert_eq!(app.game().state(), GameState::Won);
        assert_eq!(app.game().rounds_played(), 1);
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
        assert_eq!(app.message(), Some(won_message().as_str()));
    }

    #[test]
    fn a_new_app_opens_on_the_menu() {
        let app = App::new();

        assert_eq!(app.screen(), Screen::Menu);
        assert_eq!(app.menu_selection(), MenuEntry::StartGame);
    }

    #[test]
    fn menu_selection_wraps_in_both_directions() {
        let mut app = app_with("crane");
        app.open_menu();

        app.select_previous_menu_entry();
        assert_eq!(app.menu_selection(), MenuEntry::Quit);

        app.select_next_menu_entry();
        assert_eq!(app.menu_selection(), MenuEntry::StartGame);
    }

    #[test]
    fn start_game_resets_the_round_and_switches_to_the_game() {
        let mut app = app_with("crane");
        type_word(&mut app, "slate");
        app.submit();
        app.open_menu();

        assert!(!app.activate_menu_entry());

        assert_eq!(app.screen(), Screen::Game);
        assert_eq!(app.game().state(), GameState::Playing);
        assert_eq!(app.game().rounds_played(), 0);
        assert_eq!(app.typed(), "");
        assert_eq!(app.message(), None);
    }

    #[test]
    fn activating_credits_and_repository_switch_screens() {
        let mut app = app_with("crane");
        app.open_menu();

        app.select_next_menu_entry();
        assert!(!app.activate_menu_entry());
        assert_eq!(app.screen(), Screen::Credits);

        app.open_menu();
        app.select_next_menu_entry();
        assert!(!app.activate_menu_entry());
        assert_eq!(app.screen(), Screen::Repository);

        app.open_menu();
        assert_eq!(app.screen(), Screen::Menu);
    }

    #[test]
    fn activating_quit_asks_to_quit() {
        let mut app = app_with("crane");
        app.open_menu();
        app.select_previous_menu_entry();

        assert!(app.activate_menu_entry());
    }
}
