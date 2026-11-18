//! Embedded word lists, guess validation, and answer selection.
//!
//! One answer list and one allowed list are embedded per supported word size
//! with [`include_str!`], so no file I/O happens at runtime. The lists are
//! parsed lazily, once, on first use; a malformed list makes that first use
//! panic with the offending source and word, since the embedded data can only
//! be wrong at compile time.

use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::game::{Answer, MAX_WORD_LEN, MIN_WORD_LEN};

/// The embedded answer and allowed lists, one pair per supported word size.
const EMBEDDED: [(usize, &str, &str); 6] = [
    (
        3,
        include_str!("../assets/answers-3.txt"),
        include_str!("../assets/allowed-3.txt"),
    ),
    (
        4,
        include_str!("../assets/answers-4.txt"),
        include_str!("../assets/allowed-4.txt"),
    ),
    (
        5,
        include_str!("../assets/answers-5.txt"),
        include_str!("../assets/allowed-5.txt"),
    ),
    (
        6,
        include_str!("../assets/answers-6.txt"),
        include_str!("../assets/allowed-6.txt"),
    ),
    (
        7,
        include_str!("../assets/answers-7.txt"),
        include_str!("../assets/allowed-7.txt"),
    ),
    (
        8,
        include_str!("../assets/answers-8.txt"),
        include_str!("../assets/allowed-8.txt"),
    ),
];

struct Dictionary {
    word_len: usize,
    answers: Vec<&'static str>,
    allowed: HashSet<&'static str>,
}

static DICTIONARIES: LazyLock<Vec<Dictionary>> = LazyLock::new(build_dictionaries);

fn dictionaries() -> &'static [Dictionary] {
    &DICTIONARIES
}

/// The word lists of one size; panics for a size outside the supported range,
/// which only a programming error can request.
fn dictionary(word_len: usize) -> &'static Dictionary {
    assert!(
        (MIN_WORD_LEN..=MAX_WORD_LEN).contains(&word_len),
        "no embedded word lists for size {word_len}"
    );
    dictionaries()
        .iter()
        .find(|dictionary| dictionary.word_len == word_len)
        .expect("every supported size has an embedded dictionary")
}

fn build_dictionaries() -> Vec<Dictionary> {
    EMBEDDED
        .iter()
        .map(|&(word_len, answers, allowed)| build_dictionary(word_len, answers, allowed))
        .collect()
}

fn build_dictionary(word_len: usize, answers: &'static str, allowed: &'static str) -> Dictionary {
    let answers = parse_list(answers, word_len, "answers");
    let allowed = parse_list(allowed, word_len, "allowed");
    let allowed: HashSet<&'static str> = allowed.into_iter().collect();

    for answer in &answers {
        assert!(
            allowed.contains(answer),
            "answer {answer:?} is missing from the {word_len}-letter allowed list"
        );
    }

    Dictionary {
        word_len,
        answers,
        allowed,
    }
}

/// Parses one word per line, panicking on an empty or malformed list.
///
/// `role` names the list in panic messages (`answers` or `allowed`).
fn parse_list(text: &'static str, word_len: usize, role: &str) -> Vec<&'static str> {
    let words: Vec<&'static str> = text.lines().collect();
    assert!(
        !words.is_empty(),
        "the {word_len}-letter {role} list must not be empty"
    );
    for word in &words {
        assert!(
            is_lowercase_letters(word, word_len),
            "the {word_len}-letter {role} list contains an invalid word: {word:?}"
        );
    }
    words
}

fn is_lowercase_letters(word: &str, word_len: usize) -> bool {
    word.len() == word_len && word.bytes().all(|byte| byte.is_ascii_lowercase())
}

/// Whether `word` may be submitted as a guess in a game of size `word_len`.
///
/// Only words from that size's embedded allowed list are accepted, which also
/// keeps out wrong-length and non-alphabetic input.
pub fn is_valid_guess(word: &str, word_len: usize) -> bool {
    word.len() == word_len && dictionary(word_len).allowed.contains(word)
}

/// Draws a random answer from the embedded answer list of size `word_len`.
pub fn random_answer(word_len: usize) -> Answer {
    draw_answer(random_seed(), word_len)
}

/// Deterministic core of [`random_answer`], used by tests to draw with a
/// fixed seed.
fn draw_answer(seed: u64, word_len: usize) -> Answer {
    let answers = &dictionary(word_len).answers;
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
    fn every_size_has_non_empty_lists() {
        for &(word_len, _, _) in &EMBEDDED {
            let dictionary = dictionary(word_len);

            assert!(
                !dictionary.answers.is_empty(),
                "size {word_len} has no answers"
            );
            assert!(
                !dictionary.allowed.is_empty(),
                "size {word_len} has no allowed words"
            );
        }
    }

    #[test]
    fn every_word_has_its_list_length_and_is_lowercase() {
        for &(word_len, answers, allowed) in &EMBEDDED {
            for word in answers.lines() {
                assert!(
                    is_lowercase_letters(word, word_len),
                    "size {word_len} invalid answer word: {word:?}"
                );
            }
            for word in allowed.lines() {
                assert!(
                    is_lowercase_letters(word, word_len),
                    "size {word_len} invalid allowed word: {word:?}"
                );
            }
        }
    }

    #[test]
    fn every_answer_is_an_allowed_guess_at_its_size() {
        for &(word_len, _, _) in &EMBEDDED {
            let dictionary = dictionary(word_len);

            for answer in &dictionary.answers {
                assert!(
                    dictionary.allowed.contains(answer),
                    "size {word_len}: answer {answer:?} is not an allowed guess"
                );
            }
        }
    }

    #[test]
    fn drawn_answer_comes_from_the_list_of_its_size() {
        for &(word_len, _, _) in &EMBEDDED {
            let dictionary = dictionary(word_len);

            for seed in 0..64 {
                let answer = draw_answer(seed, word_len);
                assert_eq!(answer.len(), word_len);
                assert!(
                    dictionary.answers.contains(&answer.as_str()),
                    "size {word_len}: drawn answer {:?} is not in the answer list",
                    answer.as_str()
                );
                assert!(is_valid_guess(answer.as_str(), word_len));
            }
        }
    }

    #[test]
    fn is_valid_guess_rejects_non_words_and_wrong_sizes() {
        assert!(is_valid_guess("cigar", 5));
        assert!(!is_valid_guess("cigar", 6));
        assert!(!is_valid_guess("abcdef", 5));
        assert!(!is_valid_guess("zzzzz", 5));
        assert!(is_valid_guess("cat", 3));
        assert!(!is_valid_guess("cat", 4));
    }
}
