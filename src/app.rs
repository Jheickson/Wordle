//! Application state (the model).
//!
//! [`App`] owns the [`Game`], the word the player is currently typing, and a
//! transient message line. All input semantics live here as plain methods so
//! they can be unit-tested without a terminal; the UI only renders the
//! resulting state and the event loop only translates keys into these calls.

use std::time::SystemTime;

use crate::game::{
    Answer, Game, GameError, GameState, DEFAULT_ROUNDS, DEFAULT_WORD_LEN, MAX_ROUNDS, MAX_WORD_LEN,
    MIN_ROUNDS, MIN_WORD_LEN,
};
use crate::stats::{self, GameRecord, Stats};
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
    /// Shows the completed-game stats.
    Stats,
    /// Shows who built the game.
    Credits,
    /// Shows where the source lives.
    Repository,
    /// Exits the app.
    Quit,
}

impl MenuEntry {
    /// Every entry in display order.
    pub const ALL: [MenuEntry; 5] = [
        MenuEntry::StartGame,
        MenuEntry::Stats,
        MenuEntry::Credits,
        MenuEntry::Repository,
        MenuEntry::Quit,
    ];

    /// The label shown on screen.
    pub fn title(self) -> &'static str {
        match self {
            MenuEntry::StartGame => "Start game",
            MenuEntry::Stats => "Stats",
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
    /// The completed-game stats.
    Stats,
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
    /// Number of letters per word for the next game, chosen on the menu.
    word_size: usize,
    /// Number of rounds for the next game, chosen on the menu.
    max_rounds: usize,
    /// Every completed game, loaded at startup.
    stats: Stats,
    /// When the current game started, for its recorded duration.
    game_started: SystemTime,
    /// First guess submitted in the current game, for the opening-word stat.
    first_guess: Option<String>,
}

impl App {
    /// Starts a new game with a randomly drawn answer and opens on the menu.
    ///
    /// The stats database is opened at its default location; when it cannot be
    /// opened (permissions, corruption, newer schema), the session keeps stats
    /// in memory instead of failing to start.
    pub fn new() -> Self {
        let mut app = Self::with_answer(words::random_answer(DEFAULT_WORD_LEN));
        app.screen = Screen::Menu;
        app.stats = stats::open_default().unwrap_or_default();
        app
    }

    /// Starts a new game with a fixed answer, opening straight onto the game
    /// screen, for tests and deterministic setups.
    ///
    /// The app's word size is set to the answer's length and its round limit
    /// to [`DEFAULT_ROUNDS`], so a following [`App::new_game`] draws a game
    /// with the same settings. Stats start empty and stay in memory: no
    /// database is read or written because [`App::new`] is the only
    /// constructor that opens one.
    pub fn with_answer(answer: Answer) -> Self {
        let word_size = answer.len();
        Self {
            game: Game::new(answer, DEFAULT_ROUNDS),
            typed: String::new(),
            message: None,
            screen: Screen::Game,
            menu_selection: 0,
            word_size,
            max_rounds: DEFAULT_ROUNDS,
            stats: Stats::default(),
            game_started: SystemTime::now(),
            first_guess: None,
        }
    }

    /// The game being played.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Starts a fresh game with a newly drawn answer of the current word
    /// size and with the current round limit.
    ///
    /// The board, the typed word, and the message line are reset, so the app
    /// looks exactly as it does right after startup.
    pub fn new_game(&mut self) {
        self.game = Game::new(words::random_answer(self.word_size), self.max_rounds);
        self.typed.clear();
        self.message = None;
        self.game_started = SystemTime::now();
        self.first_guess = None;
    }

    /// Every completed game and its aggregates.
    pub fn stats(&self) -> &Stats {
        &self.stats
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

    /// The number of letters per word the next game will use.
    pub fn word_size(&self) -> usize {
        self.word_size
    }

    /// Shrinks the word size shown on the Start game entry by one letter.
    ///
    /// The size clamps at [`MIN_WORD_LEN`]. The size keys only act while the
    /// menu cursor rests on Start game, so other entries ignore them.
    pub fn decrease_word_size(&mut self) {
        self.adjust_word_size(-1);
    }

    /// Grows the word size shown on the Start game entry by one letter.
    ///
    /// The size clamps at [`MAX_WORD_LEN`]. The size keys only act while the
    /// menu cursor rests on Start game, so other entries ignore them.
    pub fn increase_word_size(&mut self) {
        self.adjust_word_size(1);
    }

    fn adjust_word_size(&mut self, delta: isize) {
        if !self.start_entry_hovered() {
            return;
        }
        self.word_size = (self.word_size as isize + delta)
            .clamp(MIN_WORD_LEN as isize, MAX_WORD_LEN as isize) as usize;
    }

    /// The number of rounds the next game will allow.
    pub fn max_rounds(&self) -> usize {
        self.max_rounds
    }

    /// Shrinks the guess limit shown on the Start game entry by one round.
    ///
    /// The limit clamps at [`MIN_ROUNDS`]. The limit keys only act while the
    /// menu cursor rests on Start game, so other entries ignore them.
    pub fn decrease_max_rounds(&mut self) {
        self.adjust_max_rounds(-1);
    }

    /// Grows the guess limit shown on the Start game entry by one round.
    ///
    /// The limit clamps at [`MAX_ROUNDS`]. The limit keys only act while the
    /// menu cursor rests on Start game, so other entries ignore them.
    pub fn increase_max_rounds(&mut self) {
        self.adjust_max_rounds(1);
    }

    fn adjust_max_rounds(&mut self, delta: isize) {
        if !self.start_entry_hovered() {
            return;
        }
        self.max_rounds = (self.max_rounds as isize + delta)
            .clamp(MIN_ROUNDS as isize, MAX_ROUNDS as isize) as usize;
    }

    /// Whether the menu cursor rests on the Start game entry.
    fn start_entry_hovered(&self) -> bool {
        self.screen == Screen::Menu && self.menu_selection() == MenuEntry::StartGame
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
            MenuEntry::Stats => self.screen = Screen::Stats,
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
    /// Non-alphabetic keys and overflow beyond the game's word size are
    /// ignored. Input only acts while the game is still being played, so a
    /// finished game keeps its end-state message on screen. Typing clears any
    /// stale message.
    pub fn input_letter(&mut self, letter: char) {
        if self.game.state() != GameState::Playing
            || !letter.is_ascii_alphabetic()
            || self.typed.len() >= self.game.word_len()
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

        if self.typed.len() < self.game.word_len() {
            self.message = Some(TOO_SHORT_MESSAGE.to_string());
            return;
        }

        if !words::is_valid_guess(&self.typed, self.game.word_len()) {
            self.message = Some(UNKNOWN_WORD_MESSAGE.to_string());
            return;
        }

        // Move the word out so the game can borrow it while `self` stays usable
        // for the follow-up state change.
        let word = std::mem::take(&mut self.typed);
        let result = self.game.submit_guess(&word).map(|_| ());

        match result {
            Ok(()) => {
                if self.first_guess.is_none() {
                    self.first_guess = Some(word.clone());
                }
                self.message = match self.game.state() {
                    GameState::Playing => None,
                    GameState::Won => {
                        self.record_finished_game(true);
                        Some(won_message())
                    }
                    GameState::Lost => {
                        self.record_finished_game(false);
                        Some(self.lost_message())
                    }
                };
            }
            // Unreachable: the checks above guarantee a playing game and a
            // word-length valid word. Restore the buffer so no input is lost.
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

    /// Appends the just-finished game to [`Stats`], which writes it to the
    /// database when one is open.
    fn record_finished_game(&mut self, won: bool) {
        let duration_secs = SystemTime::now()
            .duration_since(self.game_started)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        self.stats.push(GameRecord {
            timestamp: stats::unix_now(),
            word_size: self.game.word_len(),
            round_limit: self.game.max_rounds(),
            rounds_played: self.game.rounds_played(),
            won,
            duration_secs,
            answer: self.game.answer().as_str().to_owned(),
            first_guess: self.first_guess.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DEFAULT_ROUNDS;

    fn app_with(answer: &str) -> App {
        App::with_answer(Answer::new(answer).expect("test answer is a supported word"))
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

        for _ in 0..DEFAULT_ROUNDS {
            type_word(&mut app, "slate");
            app.submit();
            assert_eq!(app.typed(), "");
        }

        assert_eq!(app.game().state(), GameState::Lost);
        assert_eq!(app.game().rounds_played(), DEFAULT_ROUNDS);
    }

    #[test]
    fn losing_message_reveals_the_answer() {
        let mut app = app_with("swamp");

        for _ in 0..DEFAULT_ROUNDS {
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
        for _ in 0..DEFAULT_ROUNDS {
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
            words::is_valid_guess(&answer, app.game().word_len()),
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
    fn word_size_changes_only_while_start_game_is_hovered() {
        let mut app = app_with("crane");
        app.open_menu();
        assert_eq!(app.word_size(), 5);

        app.increase_word_size();
        assert_eq!(app.word_size(), 6);
        app.decrease_word_size();
        app.decrease_word_size();
        assert_eq!(app.word_size(), 4);

        app.select_next_menu_entry();
        app.increase_word_size();
        assert_eq!(app.word_size(), 4, "other entries ignore the size keys");
    }

    #[test]
    fn word_size_clamps_at_the_bounds() {
        let mut app = app_with("crane");
        app.open_menu();

        for _ in 0..10 {
            app.increase_word_size();
        }
        assert_eq!(app.word_size(), MAX_WORD_LEN);

        for _ in 0..20 {
            app.decrease_word_size();
        }
        assert_eq!(app.word_size(), MIN_WORD_LEN);
    }

    #[test]
    fn guess_limit_changes_only_while_start_game_is_hovered() {
        let mut app = app_with("crane");
        app.open_menu();
        assert_eq!(app.max_rounds(), DEFAULT_ROUNDS);

        app.increase_max_rounds();
        assert_eq!(app.max_rounds(), DEFAULT_ROUNDS + 1);
        app.decrease_max_rounds();
        app.decrease_max_rounds();
        assert_eq!(app.max_rounds(), DEFAULT_ROUNDS - 1);

        app.select_next_menu_entry();
        app.increase_max_rounds();
        assert_eq!(
            app.max_rounds(),
            DEFAULT_ROUNDS - 1,
            "other entries ignore the limit keys"
        );
    }

    #[test]
    fn guess_limit_clamps_at_the_bounds() {
        let mut app = app_with("crane");
        app.open_menu();

        for _ in 0..20 {
            app.increase_max_rounds();
        }
        assert_eq!(app.max_rounds(), MAX_ROUNDS);

        for _ in 0..30 {
            app.decrease_max_rounds();
        }
        assert_eq!(app.max_rounds(), MIN_ROUNDS);
    }

    #[test]
    fn starting_a_game_uses_the_selected_guess_limit() {
        let mut app = app_with("crane");
        app.open_menu();
        app.decrease_max_rounds();
        app.decrease_max_rounds();
        assert_eq!(app.max_rounds(), DEFAULT_ROUNDS - 2);

        assert!(!app.activate_menu_entry());

        assert_eq!(app.screen(), Screen::Game);
        assert_eq!(app.game().max_rounds(), DEFAULT_ROUNDS - 2);
    }

    #[test]
    fn new_game_keeps_the_selected_guess_limit() {
        let mut app = app_with("crane");
        app.open_menu();
        app.increase_max_rounds();
        app.increase_max_rounds();
        app.activate_menu_entry();

        app.new_game();

        assert_eq!(app.max_rounds(), DEFAULT_ROUNDS + 2);
        assert_eq!(app.game().max_rounds(), DEFAULT_ROUNDS + 2);
    }

    #[test]
    fn starting_a_game_uses_the_selected_word_size() {
        let mut app = app_with("crane");
        app.open_menu();
        for _ in 0..3 {
            app.increase_word_size();
        }
        assert_eq!(app.word_size(), MAX_WORD_LEN);

        assert!(!app.activate_menu_entry());

        assert_eq!(app.screen(), Screen::Game);
        assert_eq!(app.game().word_len(), MAX_WORD_LEN);

        let answer = app.game().answer().as_str().to_string();
        assert_eq!(answer.len(), MAX_WORD_LEN);
        type_word(&mut app, &answer);
        app.submit();
        assert_eq!(app.game().state(), GameState::Won);
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
    fn activating_stats_credits_and_repository_switch_screens() {
        let mut app = app_with("crane");
        app.open_menu();

        app.select_next_menu_entry();
        assert!(!app.activate_menu_entry());
        assert_eq!(app.screen(), Screen::Stats);

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

    #[test]
    fn a_won_game_is_recorded_in_the_stats() {
        let mut app = app_with("crane");

        type_word(&mut app, "crane");
        app.submit();

        assert_eq!(app.stats().games_played(), 1);
        let record = app.stats().recent(1).next().unwrap();
        assert!(record.won);
        assert_eq!(record.word_size, 5);
        assert_eq!(record.round_limit, DEFAULT_ROUNDS);
        assert_eq!(record.rounds_played, 1);
        assert_eq!(record.answer, "crane");
    }

    #[test]
    fn a_lost_game_is_recorded_in_the_stats() {
        let mut app = app_with("crane");

        for word in ["slate", "brick", "fudge", "pylon", "whelp", "anvil"] {
            type_word(&mut app, word);
            app.submit();
        }

        assert_eq!(app.game().state(), GameState::Lost);
        assert_eq!(app.stats().games_played(), 1);
        assert!(!app.stats().recent(1).next().unwrap().won);
    }

    #[test]
    fn a_game_records_the_first_guess_as_its_opening_word() {
        let mut app = app_with("crane");

        type_word(&mut app, "slate");
        app.submit();
        type_word(&mut app, "crane");
        app.submit();

        let record = app.stats().recent(1).next().unwrap();
        assert_eq!(record.first_guess.as_deref(), Some("slate"));
    }

    #[test]
    fn every_game_records_its_own_opening_word() {
        let mut app = app_with("crane");
        type_word(&mut app, "slate");
        app.submit();
        type_word(&mut app, "crane");
        app.submit();

        app.new_game();
        let answer = app.game().answer().as_str().to_owned();
        type_word(&mut app, &answer);
        app.submit();

        let openings: Vec<Option<&str>> = app
            .stats()
            .recent(2)
            .map(|record| record.first_guess.as_deref())
            .collect();
        assert_eq!(openings, vec![Some(answer.as_str()), Some("slate")]);
    }

    #[test]
    fn unfinished_games_are_not_recorded() {
        let mut app = app_with("crane");

        type_word(&mut app, "slate");
        app.submit();
        app.open_menu();

        assert_eq!(app.stats().games_played(), 0);
    }

    #[test]
    fn completed_games_persist_to_the_stats_database() {
        let path =
            std::env::temp_dir().join(format!("wordle-app-stats-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut app = app_with("crane");
        app.stats = Stats::open(&path).unwrap();

        type_word(&mut app, "crane");
        app.submit();

        assert_eq!(Stats::open(&path).unwrap().games_played(), 1);
        for suffix in ["", "-wal", "-shm"] {
            let mut file = path.as_os_str().to_owned();
            file.push(suffix);
            let _ = std::fs::remove_file(std::path::PathBuf::from(file));
        }
    }
}
