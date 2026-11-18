//! Embedded word lists, guess validation, and answer selection.
//!
//! Both lists are compiled into the binary with [`include_str!`], so no file
//! I/O happens at runtime. The lists are parsed lazily, once, on first use;
//! a malformed list makes that first use panic with the offending source and
//! word, since the embedded data can only be wrong at compile time.

use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::game::{Answer, WORD_LEN};

/// The words a game may choose as its secret answer.
const ANSWERS: &str = include_str!("../assets/answers.txt");

/// Every word the player is allowed to guess (a superset of [`ANSWERS`]).
const ALLOWED: &str = include_str!("../assets/allowed.txt");

struct Dictionary {
    answers: Vec<&'static str>,
    allowed: HashSet<&'static str>,
}

static DICTIONARY: LazyLock<Dictionary> = LazyLock::new(build_dictionary);

fn dictionary() -> &'static Dictionary {
    &DICTIONARY
}

fn build_dictionary() -> Dictionary {
    let answers = parse_list(ANSWERS, "assets/answers.txt");
    let allowed = parse_list(ALLOWED, "assets/allowed.txt");
    let allowed: HashSet<&'static str> = allowed.into_iter().collect();

    for answer in &answers {
        assert!(
            allowed.contains(answer),
            "answer {answer:?} is missing from assets/allowed.txt"
        );
    }

    Dictionary { answers, allowed }
}

/// Parses one word per line, panicking on an empty or malformed list.
fn parse_list(text: &'static str, source: &str) -> Vec<&'static str> {
    let words: Vec<&'static str> = text.lines().collect();
    assert!(!words.is_empty(), "{source} must not be empty");
    for word in &words {
        assert!(
            is_five_lowercase_letters(word),
            "{source} contains an invalid word: {word:?}"
        );
    }
    words
}

fn is_five_lowercase_letters(word: &str) -> bool {
    word.len() == WORD_LEN && word.bytes().all(|byte| byte.is_ascii_lowercase())
}

/// Whether `word` may be submitted as a guess.
///
/// Only words from the embedded allowed list are accepted, which also keeps
/// out wrong-length and non-alphabetic input.
pub fn is_valid_guess(word: &str) -> bool {
    word.len() == WORD_LEN && dictionary().allowed.contains(word)
}

/// Draws a random answer from the embedded answer list.
pub fn random_answer() -> Answer {
    draw_answer(random_seed())
}

/// Deterministic core of [`random_answer`], used by tests to draw with a
/// fixed seed.
fn draw_answer(seed: u64) -> Answer {
    let answers = &dictionary().answers;
    let index = (xorshift64star(seed) % answers.len() as u64) as usize;
    Answer::new(answers[index]).expect("embedded answers were validated")
}

/// Mixes the clock and a draw counter into a fresh PRNG seed.
fn random_seed() -> u64 {
    static DRAWS: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0);
    let draws = DRAWS.fetch_add(1, Ordering::Relaxed);

    mix64(nanos ^ draws.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Finalizer of splitmix64; spreads seed bits before the xorshift steps.
fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

/// xorshift64*: a tiny PRNG so randomness needs no extra crate.
fn xorshift64star(mut state: u64) -> u64 {
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_lists_are_non_empty() {
        let dictionary = dictionary();

        assert!(!dictionary.answers.is_empty());
        assert!(!dictionary.allowed.is_empty());
    }

    #[test]
    fn every_word_is_five_lowercase_letters() {
        for word in ANSWERS.lines() {
            assert!(
                is_five_lowercase_letters(word),
                "invalid answer word: {word:?}"
            );
        }
        for word in ALLOWED.lines() {
            assert!(
                is_five_lowercase_letters(word),
                "invalid allowed word: {word:?}"
            );
        }
    }

    #[test]
    fn every_answer_is_an_allowed_guess() {
        let dictionary = dictionary();

        for answer in &dictionary.answers {
            assert!(
                dictionary.allowed.contains(answer),
                "answer {answer:?} is not an allowed guess"
            );
        }
    }

    #[test]
    fn drawn_answer_comes_from_answer_list() {
        let dictionary = dictionary();

        for seed in 0..256 {
            let answer = draw_answer(seed);
            assert!(
                dictionary.answers.contains(&answer.as_str()),
                "drawn answer {:?} is not in the answer list",
                answer.as_str()
            );
            assert!(is_valid_guess(answer.as_str()));
        }
    }

    #[test]
    fn is_valid_guess_rejects_non_words() {
        assert!(is_valid_guess("cigar"));
        assert!(!is_valid_guess("abcdef"));
        assert!(!is_valid_guess("zzzzz"));
    }
}
