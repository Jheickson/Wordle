//! Core Wordle rules, independent of the terminal UI.
//!
//! The module owns the domain vocabulary: [`Answer`], [`Guess`],
//! [`Evaluation`], [`Board`], [`Round`], and [`GameState`]. It performs no I/O
//! so the same rules can be driven by the TUI or by tests.

use std::fmt;

/// Number of letters in every word.
pub const WORD_LEN: usize = 5;

/// Number of rounds a game lasts.
pub const MAX_ROUNDS: usize = 6;

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
pub type Evaluation = [TileState; WORD_LEN];

/// Error returned when constructing a word of the wrong length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordError {
    WrongLength,
}

impl fmt::Display for WordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength => write!(f, "word must be exactly {WORD_LEN} letters"),
        }
    }
}

impl std::error::Error for WordError {}

/// Error returned when a guess cannot be submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameError {
    /// The guess was not a valid five-letter word.
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

/// The secret five-letter word for a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    word: String,
}

impl Answer {
    /// Creates an answer from a five-letter word.
    pub fn new(word: &str) -> Result<Self, WordError> {
        validate_word(word)?;
        Ok(Self {
            word: word.to_owned(),
        })
    }

    /// The answer's letters.
    pub fn as_str(&self) -> &str {
        &self.word
    }
}

/// A five-letter word submitted by the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guess {
    word: String,
}

impl Guess {
    /// Creates a guess from a five-letter word.
    pub fn new(word: &str) -> Result<Self, WordError> {
        validate_word(word)?;
        Ok(Self {
            word: word.to_owned(),
        })
    }

    /// The guess's letters.
    pub fn as_str(&self) -> &str {
        &self.word
    }
}

fn validate_word(word: &str) -> Result<(), WordError> {
    if word.chars().count() == WORD_LEN {
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

    let mut evaluation = [TileState::Absent; WORD_LEN];
    let mut unused = [true; WORD_LEN];

    for i in 0..WORD_LEN {
        if guess_letters[i] == answer_letters[i] {
            evaluation[i] = TileState::Correct;
            unused[i] = false;
        }
    }

    for i in 0..WORD_LEN {
        if evaluation[i] == TileState::Correct {
            continue;
        }
        if let Some(j) =
            (0..WORD_LEN).find(|&j| unused[j] && answer_letters[j] == guess_letters[i])
        {
            evaluation[i] = TileState::Present;
            unused[j] = false;
        }
    }

    evaluation
}

fn letters_of(word: &str) -> [char; WORD_LEN] {
    let mut letters = ['\0'; WORD_LEN];
    for (letter, ch) in letters.iter_mut().zip(word.chars()) {
        *letter = ch;
    }
    letters
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
    pub fn evaluation(&self) -> Evaluation {
        self.evaluation
    }
}

/// The 6×5 grid of rounds played so far.
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

    /// Whether all [`MAX_ROUNDS`] rows have been filled.
    pub fn is_full(&self) -> bool {
        self.rounds.len() == MAX_ROUNDS
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
    /// [`MAX_ROUNDS`] rounds passed without a win.
    Lost,
}

/// A Wordle game with a known answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    answer: Answer,
    board: Board,
    state: GameState,
}

impl Game {
    /// Starts a new game with the given answer.
    pub fn new(answer: Answer) -> Self {
        Self {
            answer,
            board: Board::default(),
            state: GameState::Playing,
        }
    }

    /// The answer (hidden during play; meant for revealing on loss).
    pub fn answer(&self) -> &Answer {
        &self.answer
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

    /// Submits a five-letter guess.
    ///
    /// The guess is evaluated and recorded, then the game state advances:
    /// all-Correct wins, a full board without a win loses, anything else keeps
    /// playing. Invalid guesses and guesses after the game ended are rejected
    /// without changing any state.
    pub fn submit_guess(&mut self, word: &str) -> Result<&Round, GameError> {
        if self.state != GameState::Playing {
            return Err(GameError::GameOver);
        }

        let guess = Guess::new(word).map_err(GameError::InvalidGuess)?;
        let evaluation = evaluate(&self.answer, &guess);
        let won = evaluation.iter().all(|&tile| tile == TileState::Correct);

        self.board.push(Round::new(guess, evaluation));

        self.state = if won {
            GameState::Won
        } else if self.board.is_full() {
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
        Game::new(Answer::new(answer).expect("valid test answer"))
    }

    #[test]
    fn all_correct_guess_wins() {
        let mut game = game("cigar");

        let evaluation = game.submit_guess("cigar").unwrap().evaluation();

        assert_eq!(evaluation, [TileState::Correct; WORD_LEN]);
        assert_eq!(game.state(), GameState::Won);
        assert_eq!(game.rounds_played(), 1);
    }

    #[test]
    fn guess_with_no_matching_letters_is_all_absent() {
        let mut game = game("crate");

        let evaluation = game.submit_guess("moldy").unwrap().evaluation();

        assert_eq!(evaluation, [TileState::Absent; WORD_LEN]);
        assert_eq!(game.state(), GameState::Playing);
    }

    #[test]
    fn guess_repeats_only_mark_available_answer_occurrences() {
        let speed = Guess::new("speed").unwrap();

        // Both `e`s in `speed` sit in the positions of the two `e`s in
        // `sweet`, so both are Correct and neither has a spare occurrence.
        let sweet = Answer::new("sweet").unwrap();
        assert_eq!(
            evaluate(&sweet, &speed),
            [
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
            [
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
        let guess = Guess::new("bobby").unwrap();

        // `abbey` has two `b`s; the third guessed `b` has no occurrence left.
        assert_eq!(
            evaluate(&answer, &guess),
            [
                TileState::Present,
                TileState::Absent,
                TileState::Correct,
                TileState::Absent,
                TileState::Correct,
            ]
        );

        // A single guessed `b` against the doubled answer letter marks one
        // tile Present and leaves the other occurrence unused.
        let single_b = Guess::new("cabin").unwrap();
        assert_eq!(
            evaluate(&answer, &single_b),
            [
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
        let guesses = ["moldy", "sweat", "plumb", "fudge", "knock", "vixen"];

        for (index, guess) in guesses.iter().enumerate() {
            let evaluation = game.submit_guess(guess).unwrap().evaluation();
            assert_ne!(evaluation, [TileState::Correct; WORD_LEN]);
            assert_eq!(
                game.state(),
                if index + 1 == MAX_ROUNDS {
                    GameState::Lost
                } else {
                    GameState::Playing
                }
            );
        }

        assert_eq!(game.rounds_played(), MAX_ROUNDS);
        assert_eq!(game.submit_guess("cigar"), Err(GameError::GameOver));
        assert_eq!(game.rounds_played(), MAX_ROUNDS);
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
}
