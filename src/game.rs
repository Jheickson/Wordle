//! Core Wordle rules, independent of the terminal UI.
//!
//! The module owns the domain vocabulary: [`Answer`], [`Guess`],
//! [`Evaluation`], [`Board`], [`Round`], and [`GameState`]. It performs no I/O
//! so the same rules can be driven by the TUI or by tests.

use std::fmt;
use std::ops::RangeInclusive;

/// Smallest supported number of letters in a word.
pub const MIN_WORD_LEN: usize = 3;

/// Largest supported number of letters in a word.
pub const MAX_WORD_LEN: usize = 8;

/// The word size a new app starts with.
pub const DEFAULT_WORD_LEN: usize = 5;

/// Smallest selectable number of rounds a game lasts.
pub const MIN_ROUNDS: usize = 1;

/// Largest selectable number of rounds a game lasts.
pub const MAX_ROUNDS: usize = 15;

/// The number of rounds a new app starts with.
pub const DEFAULT_ROUNDS: usize = 6;

/// Classification of a single guessed letter against the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileState {
    /// Letter in the right position.
    Correct,
    /// Letter occurs elsewhere in the answer.
    Present,
    /// Letter has no remaining occurrence in the answer.
    Absent,
}

/// One tile state per position of a guess.
pub type Evaluation = Vec<TileState>;

/// Best tile state seen for each letter of the alphabet (index 0 = `A`), or
/// `None` when the letter was never guessed.
pub type KeyboardStates = [Option<TileState>; 26];

/// The position of `letter` in [`KeyboardStates`] (0 = `A`).
pub fn keyboard_index(letter: char) -> usize {
    letter.to_ascii_uppercase() as usize - 'A' as usize
}

/// Strength of a tile state when picking a keyboard key's color: a correct
/// letter outranks a present one, which outranks an absent one.
fn tile_state_rank(state: TileState) -> u8 {
    match state {
        TileState::Correct => 2,
        TileState::Present => 1,
        TileState::Absent => 0,
    }
}

/// Error returned when constructing a word of the wrong length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordError {
    WrongLength,
}

impl fmt::Display for WordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength => write!(f, "word has the wrong number of letters"),
        }
    }
}

impl std::error::Error for WordError {}

/// Error returned when a guess cannot be submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameError {
    /// The guess was not a word of the game's word size.
    InvalidGuess(WordError),
    /// The game has already ended (won or lost).
    GameOver,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGuess(error) => error.fmt(f),
            Self::GameOver => write!(f, "the game is already over"),
        }
    }
}

impl std::error::Error for GameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidGuess(error) => Some(error),
            Self::GameOver => None,
        }
    }
}

/// The secret word for a game, of any supported word size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    word: String,
}

impl Answer {
    /// Creates an answer from a word of a supported size (3 to 8 letters).
    pub fn new(word: &str) -> Result<Self, WordError> {
        validate_word_length(word, MIN_WORD_LEN..=MAX_WORD_LEN)?;
        Ok(Self {
            word: word.to_owned(),
        })
    }

    /// The answer's letters.
    pub fn as_str(&self) -> &str {
        &self.word
    }

    /// Number of letters in the answer: the game's word size.
    pub fn len(&self) -> usize {
        self.word.len()
    }
}

/// A word submitted by the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guess {
    word: String,
}

impl Guess {
    /// Creates a guess from a word that must be `word_len` letters long.
    pub fn new(word: &str, word_len: usize) -> Result<Self, WordError> {
        validate_word_length(word, word_len..=word_len)?;
        Ok(Self {
            word: word.to_owned(),
        })
    }

    /// The guess's letters.
    pub fn as_str(&self) -> &str {
        &self.word
    }
}

/// Rejects a word whose length falls outside `allowed`.
fn validate_word_length(word: &str, allowed: RangeInclusive<usize>) -> Result<(), WordError> {
    if allowed.contains(&word.chars().count()) {
        Ok(())
    } else {
        Err(WordError::WrongLength)
    }
}

/// Evaluates `guess` against `answer`.
///
/// The evaluation is frequency-aware: each position of the answer can mark at
/// most one tile. Exact matches are marked first, then every remaining guess
/// letter consumes a still-unused occurrence in the answer if one exists.
pub fn evaluate(answer: &Answer, guess: &Guess) -> Evaluation {
    let answer_letters = letters_of(answer.as_str());
    let guess_letters = letters_of(guess.as_str());
    let word_len = answer_letters.len();

    let mut evaluation = vec![TileState::Absent; word_len];
    let mut unused = vec![true; word_len];

    for i in 0..word_len {
        if guess_letters[i] == answer_letters[i] {
            evaluation[i] = TileState::Correct;
            unused[i] = false;
        }
    }

    for i in 0..word_len {
        if evaluation[i] == TileState::Correct {
            continue;
        }
        if let Some(j) =
            (0..word_len).find(|&j| unused[j] && answer_letters[j] == guess_letters[i])
        {
            evaluation[i] = TileState::Present;
            unused[j] = false;
        }
    }

    evaluation
}

fn letters_of(word: &str) -> Vec<char> {
    word.chars().collect()
}

/// One attempt: the submitted guess and the evaluation it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    guess: Guess,
    evaluation: Evaluation,
}

impl Round {
    fn new(guess: Guess, evaluation: Evaluation) -> Self {
        Self { guess, evaluation }
    }

    /// The word submitted this round.
    pub fn guess(&self) -> &Guess {
        &self.guess
    }

    /// The tile state for each position of the guess.
    pub fn evaluation(&self) -> &Evaluation {
        &self.evaluation
    }
}

/// The grid of rounds played so far: the game's round limit of rows by its
/// word size of columns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Board {
    rounds: Vec<Round>,
}

impl Board {
    /// Rounds played so far, in submission order.
    pub fn rounds(&self) -> &[Round] {
        &self.rounds
    }

    /// The round at `index`, if it has been played.
    pub fn round(&self, index: usize) -> Option<&Round> {
        self.rounds.get(index)
    }

    /// Number of rounds played.
    pub fn rounds_played(&self) -> usize {
        self.rounds.len()
    }

    /// Whether `max_rounds` rows have been filled.
    pub fn is_full(&self, max_rounds: usize) -> bool {
        self.rounds.len() >= max_rounds
    }

    /// The best tile state seen for each letter across all played rounds.
    ///
    /// A letter keeps the strongest signal it ever produced, so a correct
    /// guess outranks a present one, which outranks absent.
    pub fn keyboard_states(&self) -> KeyboardStates {
        let mut states = [None; 26];
        for round in &self.rounds {
            for (position, letter) in round.guess.word.chars().enumerate() {
                let index = keyboard_index(letter);
                let candidate = round.evaluation[position];
                let covered = match states[index] {
                    Some(seen) => tile_state_rank(seen) >= tile_state_rank(candidate),
                    None => false,
                };
                if !covered {
                    states[index] = Some(candidate);
                }
            }
        }
        states
    }

    fn push(&mut self, round: Round) {
        debug_assert!(self.rounds.len() < MAX_ROUNDS);
        self.rounds.push(round);
    }
}

/// Lifecycle of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// The player may still submit guesses.
    Playing,
    /// A guess evaluated all-Correct.
    Won,
    /// The round limit passed without a win.
    Lost,
}

/// A Wordle game with a known answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    answer: Answer,
    board: Board,
    state: GameState,
    max_rounds: usize,
}

impl Game {
    /// Starts a new game with the given answer and round limit.
    pub fn new(answer: Answer, max_rounds: usize) -> Self {
        assert!(
            (MIN_ROUNDS..=MAX_ROUNDS).contains(&max_rounds),
            "round limit {max_rounds} is outside {MIN_ROUNDS}..={MAX_ROUNDS}"
        );
        Self {
            answer,
            board: Board::default(),
            state: GameState::Playing,
            max_rounds,
        }
    }

    /// The answer (hidden during play; meant for revealing on loss).
    pub fn answer(&self) -> &Answer {
        &self.answer
    }

    /// Number of letters in every word of this game.
    pub fn word_len(&self) -> usize {
        self.answer.len()
    }

    /// Number of rounds the game lasts.
    pub fn max_rounds(&self) -> usize {
        self.max_rounds
    }

    /// Current game state.
    pub fn state(&self) -> GameState {
        self.state
    }

    /// The board of played rounds.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Number of rounds played so far.
    pub fn rounds_played(&self) -> usize {
        self.board.rounds_played()
    }

    /// The best tile state seen for each letter across all played rounds.
    pub fn keyboard_states(&self) -> KeyboardStates {
        self.board.keyboard_states()
    }

    /// Submits a guess of the game's word size.
    ///
    /// The guess is evaluated and recorded, then the game state advances:
    /// all-Correct wins, a full board without a win loses, anything else keeps
    /// playing. Invalid guesses and guesses after the game ended are rejected
    /// without changing any state.
    pub fn submit_guess(&mut self, word: &str) -> Result<&Round, GameError> {
        if self.state != GameState::Playing {
            return Err(GameError::GameOver);
        }

        let guess = Guess::new(word, self.word_len()).map_err(GameError::InvalidGuess)?;
        let evaluation = evaluate(&self.answer, &guess);
        let won = evaluation.iter().all(|&tile| tile == TileState::Correct);

        self.board.push(Round::new(guess, evaluation));

        self.state = if won {
            GameState::Won
        } else if self.board.is_full(self.max_rounds) {
            GameState::Lost
        } else {
            GameState::Playing
        };

        Ok(self
            .board
            .rounds()
            .last()
            .expect("the round was just recorded"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(answer: &str) -> Game {
        Game::new(Answer::new(answer).expect("valid test answer"), DEFAULT_ROUNDS)
    }

    #[test]
    fn all_correct_guess_wins() {
        let mut game = game("cigar");

        let evaluation = game.submit_guess("cigar").unwrap().evaluation();

        assert_eq!(evaluation, &vec![TileState::Correct; 5]);
        assert_eq!(game.state(), GameState::Won);
        assert_eq!(game.rounds_played(), 1);
    }

    #[test]
    fn guess_with_no_matching_letters_is_all_absent() {
        let mut game = game("crate");

        let evaluation = game.submit_guess("moldy").unwrap().evaluation();

        assert_eq!(evaluation, &vec![TileState::Absent; 5]);
        assert_eq!(game.state(), GameState::Playing);
    }

    #[test]
    fn guess_repeats_only_mark_available_answer_occurrences() {
        let speed = Guess::new("speed", 5).unwrap();

        // Both `e`s in `speed` sit in the positions of the two `e`s in
        // `sweet`, so both are Correct and neither has a spare occurrence.
        let sweet = Answer::new("sweet").unwrap();
        assert_eq!(
            evaluate(&sweet, &speed),
            vec![
                TileState::Correct,
                TileState::Absent,
                TileState::Correct,
                TileState::Correct,
                TileState::Absent,
            ]
        );

        // With only one `e` in the answer, the second guessed `e` is Absent.
        let sweat = Answer::new("sweat").unwrap();
        assert_eq!(
            evaluate(&sweat, &speed),
            vec![
                TileState::Correct,
                TileState::Absent,
                TileState::Correct,
                TileState::Absent,
                TileState::Absent,
            ]
        );
    }

    #[test]
    fn answer_repeats_only_mark_available_occurrences() {
        let answer = Answer::new("abbey").unwrap();
        let guess = Guess::new("bobby", 5).unwrap();

        // `abbey` has two `b`s; the third guessed `b` has no occurrence left.
        assert_eq!(
            evaluate(&answer, &guess),
            vec![
                TileState::Present,
                TileState::Absent,
                TileState::Correct,
                TileState::Absent,
                TileState::Correct,
            ]
        );

        // A single guessed `b` against the doubled answer letter marks one
        // tile Present and leaves the other occurrence unused.
        let single_b = Guess::new("cabin", 5).unwrap();
        assert_eq!(
            evaluate(&answer, &single_b),
            vec![
                TileState::Absent,
                TileState::Present,
                TileState::Correct,
                TileState::Absent,
                TileState::Absent,
            ]
        );
    }

    #[test]
    fn wrong_length_guess_is_rejected_without_mutating_state() {
        let mut game = game("cigar");

        assert_eq!(
            game.submit_guess("four"),
            Err(GameError::InvalidGuess(WordError::WrongLength))
        );
        assert_eq!(
            game.submit_guess("toolong"),
            Err(GameError::InvalidGuess(WordError::WrongLength))
        );

        assert_eq!(game.state(), GameState::Playing);
        assert_eq!(game.rounds_played(), 0);
        assert!(game.board().rounds().is_empty());
    }

    #[test]
    fn six_non_winning_guesses_end_the_game_as_lost() {
        let mut game = game("cigar");
        let rounds = game.max_rounds();
        let guesses = ["moldy", "sweat", "plumb", "fudge", "knock", "vixen"];

        for (index, guess) in guesses.iter().enumerate() {
            let evaluation = game.submit_guess(guess).unwrap().evaluation();
            assert_ne!(evaluation, &vec![TileState::Correct; 5]);
            assert_eq!(
                game.state(),
                if index + 1 == rounds {
                    GameState::Lost
                } else {
                    GameState::Playing
                }
            );
        }

        assert_eq!(game.rounds_played(), rounds);
        assert_eq!(game.submit_guess("cigar"), Err(GameError::GameOver));
        assert_eq!(game.rounds_played(), rounds);
    }

    #[test]
    fn a_custom_round_limit_ends_the_game_after_that_many_rounds() {
        let mut game = Game::new(Answer::new("cigar").unwrap(), 3);

        game.submit_guess("moldy").unwrap();
        game.submit_guess("sweat").unwrap();
        assert_eq!(game.state(), GameState::Playing);
        game.submit_guess("plumb").unwrap();

        assert_eq!(game.state(), GameState::Lost);
        assert_eq!(game.rounds_played(), 3);
    }

    #[test]
    fn a_one_round_game_ends_after_the_first_wrong_guess() {
        let mut game = Game::new(Answer::new("cigar").unwrap(), MIN_ROUNDS);

        game.submit_guess("moldy").unwrap();

        assert_eq!(game.state(), GameState::Lost);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn round_limits_outside_the_selectable_range_are_rejected() {
        let _ = Game::new(Answer::new("cigar").unwrap(), MAX_ROUNDS + 1);
    }

    #[test]
    fn winning_guess_ends_the_game_and_rejects_further_rounds() {
        let mut game = game("cigar");

        game.submit_guess("cigar").unwrap();
        assert_eq!(game.state(), GameState::Won);
        assert_eq!(game.submit_guess("cigar"), Err(GameError::GameOver));
        assert_eq!(game.rounds_played(), 1);
        assert_eq!(game.board().round(0).unwrap().guess().as_str(), "cigar");
    }

    #[test]
    fn keyboard_states_start_empty() {
        let game = game("cigar");

        assert_eq!(game.keyboard_states(), [None; 26]);
    }

    #[test]
    fn keyboard_states_report_one_state_per_guessed_letter() {
        let mut game = game("cigar");

        // `crate` against `cigar`: c Correct, r and a Present, t and e Absent.
        game.submit_guess("crate").unwrap();

        let states = game.keyboard_states();
        assert_eq!(states[keyboard_index('c')], Some(TileState::Correct));
        assert_eq!(states[keyboard_index('r')], Some(TileState::Present));
        assert_eq!(states[keyboard_index('a')], Some(TileState::Present));
        assert_eq!(states[keyboard_index('t')], Some(TileState::Absent));
        assert_eq!(states[keyboard_index('e')], Some(TileState::Absent));
        assert_eq!(states[keyboard_index('z')], None);
    }

    #[test]
    fn keyboard_states_upgrade_a_present_letter_to_correct() {
        let mut game = game("cigar");
        game.submit_guess("rainy").unwrap();
        assert_eq!(
            game.keyboard_states()[keyboard_index('i')],
            Some(TileState::Present)
        );

        game.submit_guess("civic").unwrap();

        assert_eq!(
            game.keyboard_states()[keyboard_index('i')],
            Some(TileState::Correct),
            "a correct letter outranks an earlier present one"
        );
    }

    #[test]
    fn keyboard_states_keep_a_correct_letter_over_its_absent_duplicate() {
        let mut game = game("cigar");

        // The first `i` is Correct; the rest of the `i`s have no occurrence
        // left, so they are Absent. The keyboard keeps the strongest signal.
        game.submit_guess("riiii").unwrap();

        let states = game.keyboard_states();
        assert_eq!(states[keyboard_index('i')], Some(TileState::Correct));
        assert_eq!(states[keyboard_index('r')], Some(TileState::Present));
    }

    #[test]
    fn games_support_other_word_sizes() {
        let mut four = game("boat");
        assert_eq!(four.word_len(), 4);

        let evaluation = four.submit_guess("boat").unwrap().evaluation();
        assert_eq!(evaluation, &vec![TileState::Correct; 4]);
        assert_eq!(four.state(), GameState::Won);

        let mut three = game("cat");
        assert_eq!(three.word_len(), 3);
        assert_eq!(
            three.submit_guess("crate"),
            Err(GameError::InvalidGuess(WordError::WrongLength))
        );
        assert_eq!(
            three.submit_guess("cast"),
            Err(GameError::InvalidGuess(WordError::WrongLength))
        );
        assert_eq!(three.submit_guess("cat").unwrap().guess().as_str(), "cat");
        assert_eq!(three.state(), GameState::Won);
    }

    #[test]
    fn answers_outside_the_supported_sizes_are_rejected() {
        assert_eq!(Answer::new("ab"), Err(WordError::WrongLength));
        assert_eq!(Answer::new("toolongword"), Err(WordError::WrongLength));
        assert!(Answer::new("mouse").is_ok());
    }
}
