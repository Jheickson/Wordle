//! Persistent record of completed games.
//!
//! [`GameRecord`] captures one finished game; [`Stats`] keeps every record and
//! derives all aggregates (win rate, streaks, guess distribution) from them,
//! so the stored data is never inconsistent with what the screens show.
//! Records live in a plain-text file under the XDG data dir; a missing or
//! corrupt file simply yields empty stats.

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::game::{MAX_ROUNDS, MAX_WORD_LEN, MIN_ROUNDS, MIN_WORD_LEN};

/// First line of the stats file; a file without it is treated as invalid.
const FORMAT_HEADER: &str = "wordle-stats v1";

/// Number of recent games the stats screen lists.
pub const RECENT_LIMIT: usize = 5;

/// One completed game: a win, or a loss by exhausting the round limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRecord {
    /// Unix time, in seconds, at which the game ended.
    pub timestamp: u64,
    /// Word size (letters per word) the game was played at.
    pub word_size: usize,
    /// Round limit (maximum guesses) the game was played with.
    pub round_limit: usize,
    /// Guesses the game took to end.
    pub rounds_played: usize,
    /// Whether the answer was found.
    pub won: bool,
    /// Seconds from the first state of the game to its end.
    pub duration_secs: u64,
    /// The answer, lowercase.
    pub answer: String,
}

impl GameRecord {
    /// Whether every field is within the game's supported ranges.
    ///
    /// A loss must have used the whole round limit; a win may not have.
    fn is_valid(&self) -> bool {
        (MIN_WORD_LEN..=MAX_WORD_LEN).contains(&self.word_size)
            && (MIN_ROUNDS..=MAX_ROUNDS).contains(&self.round_limit)
            && (1..=self.round_limit).contains(&self.rounds_played)
            && (!self.won || self.rounds_played <= self.round_limit)
            && (self.won || self.rounds_played == self.round_limit)
            && self.answer.len() == self.word_size
            && self.answer.bytes().all(|b| b.is_ascii_lowercase())
    }
}

/// Every completed game, in the order they ended.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    records: Vec<GameRecord>,
}

impl Stats {
    /// Records a completed game.
    pub fn push(&mut self, record: GameRecord) {
        self.records.push(record);
    }

    /// Number of completed games.
    pub fn games_played(&self) -> usize {
        self.records.len()
    }

    /// Number of won games.
    pub fn wins(&self) -> usize {
        self.records.iter().filter(|r| r.won).count()
    }

    /// Number of lost games.
    pub fn losses(&self) -> usize {
        self.games_played() - self.wins()
    }

    /// Whole-percent win rate, or `None` before any game was played.
    pub fn win_rate(&self) -> Option<u32> {
        if self.records.is_empty() {
            None
        } else {
            Some(self.wins() as u32 * 100 / self.games_played() as u32)
        }
    }

    /// Consecutive wins ending at the latest game.
    pub fn current_streak(&self) -> usize {
        self.records
            .iter()
            .rev()
            .take_while(|r| r.won)
            .count()
    }

    /// Most consecutive wins at any point.
    pub fn max_streak(&self) -> usize {
        let (mut best, mut run) = (0, 0);
        for record in &self.records {
            run = if record.won { run + 1 } else { 0 };
            best = best.max(run);
        }
        best.max(self.current_streak())
    }

    /// Wins per number of guesses used, index 0 = one guess. Losses are
    /// excluded, like on the original game's chart.
    pub fn win_distribution(&self) -> [usize; MAX_ROUNDS] {
        let mut distribution = [0; MAX_ROUNDS];
        for record in self.records.iter().filter(|r| r.won) {
            distribution[record.rounds_played - 1] += 1;
        }
        distribution
    }

    /// Mean guesses across wins, or `None` without a win.
    pub fn average_win_guesses(&self) -> Option<f64> {
        let wins: Vec<usize> = self
            .records
            .iter()
            .filter(|r| r.won)
            .map(|r| r.rounds_played)
            .collect();
        if wins.is_empty() {
            None
        } else {
            Some(wins.iter().sum::<usize>() as f64 / wins.len() as f64)
        }
    }

    /// The win that took the fewest guesses, or `None` without a win.
    pub fn fewest_guesses_win(&self) -> Option<&GameRecord> {
        self.records
            .iter()
            .filter(|r| r.won)
            .min_by_key(|r| r.rounds_played)
    }

    /// The win that took the least time, or `None` without a win.
    pub fn fastest_win(&self) -> Option<&GameRecord> {
        self.records
            .iter()
            .filter(|r| r.won)
            .min_by_key(|r| r.duration_secs)
    }

    /// `(word size, played, won)` for every word size that has at least one
    /// game, in word-size order.
    pub fn by_word_size(&self) -> Vec<(usize, usize, usize)> {
        (MIN_WORD_LEN..=MAX_WORD_LEN)
            .filter_map(|size| {
                let played = self
                    .records
                    .iter()
                    .filter(|r| r.word_size == size)
                    .count();
                if played == 0 {
                    None
                } else {
                    let won = self
                        .records
                        .iter()
                        .filter(|r| r.word_size == size && r.won)
                        .count();
                    Some((size, played, won))
                }
            })
            .collect()
    }

    /// The `n` most recent games, latest first.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &GameRecord> {
        self.records.iter().rev().take(n)
    }

    /// Parses stats from the file contents; an unreadable or wrongly headed
    /// file yields empty stats, and invalid lines are dropped individually.
    fn parse(text: &str) -> Self {
        let mut lines = text.lines();
        if lines.next() != Some(FORMAT_HEADER) {
            return Self::default();
        }
        let mut stats = Self::default();
        for line in lines {
            if let Some(record) = Self::parse_record(line) {
                stats.push(record);
            }
        }
        stats
    }

    /// Parses one `record` line, or `None` when it does not hold a valid game.
    fn parse_record(line: &str) -> Option<GameRecord> {
        let mut fields = line.split_whitespace();
        if fields.next()? != "record" {
            return None;
        }
        let record = GameRecord {
            timestamp: fields.next()?.parse().ok()?,
            word_size: fields.next()?.parse().ok()?,
            round_limit: fields.next()?.parse().ok()?,
            rounds_played: fields.next()?.parse().ok()?,
            won: match fields.next()? {
                "won" => true,
                "lost" => false,
                _ => return None,
            },
            duration_secs: fields.next()?.parse().ok()?,
            answer: fields.next()?.to_owned(),
        };
        if fields.next().is_some() || !record.is_valid() {
            return None;
        }
        Some(record)
    }

    /// The file's text: header line, then one line per game.
    fn serialize(&self) -> String {
        let mut text = format!("{FORMAT_HEADER}\n");
        for r in &self.records {
            let outcome = if r.won { "won" } else { "lost" };
            let _ = writeln!(
                text,
                "record {} {} {} {} {} {} {}",
                r.timestamp, r.word_size, r.round_limit, r.rounds_played, outcome,
                r.duration_secs, r.answer
            );
        }
        text
    }
}

/// Where the stats file lives: `$XDG_DATA_HOME/wordle/stats`, falling back to
/// `~/.local/share/wordle/stats`.
pub fn default_path() -> PathBuf {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from(".local/share"));
    data_home.join("wordle").join("stats")
}

/// Loads the stats from the default path; missing or corrupt files yield
/// empty stats.
pub fn load() -> Stats {
    load_from(&default_path())
}

/// Loads the stats from `path`; missing or corrupt files yield empty stats.
pub fn load_from(path: &Path) -> Stats {
    fs::read_to_string(path)
        .map(|text| Stats::parse(&text))
        .unwrap_or_default()
}

/// Saves the stats to `path`, creating parent directories.
pub fn save_to(stats: &Stats, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, stats.serialize())
}

/// Seconds elapsed since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch file per test, named by test and process, cleaned up after.
    fn scratch_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wordle-stats-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn record(won: bool, rounds_played: usize, word_size: usize) -> GameRecord {
        GameRecord {
            timestamp: 1_700_000_000,
            word_size,
            round_limit: 6,
            rounds_played,
            won,
            duration_secs: 60,
            answer: "a".repeat(word_size),
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = scratch_path("round-trip");
        let mut stats = Stats::default();
        stats.push(record(true, 3, 5));
        stats.push(record(false, 6, 4));

        save_to(&stats, &path).unwrap();
        assert_eq!(load_from(&path), stats);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_yields_empty_stats() {
        assert_eq!(load_from(Path::new("/nonexistent/wordle/stats")), Stats::default());
    }

    #[test]
    fn bad_header_yields_empty_stats() {
        let path = scratch_path("bad-header");
        std::fs::write(&path, "garbage\nrecord 1 5 6 3 won 60 crane\n").unwrap();

        assert_eq!(load_from(&path), Stats::default());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn invalid_lines_are_dropped_but_valid_ones_kept() {
        let path = scratch_path("bad-lines");
        let good = "record 1700000000 5 6 3 won 60 crane";
        std::fs::write(
            &path,
            format!("{FORMAT_HEADER}\nrecord 1 5 6 9 won 60 crane\nnope\n{good}\n"),
        )
        .unwrap();

        let stats = load_from(&path);
        assert_eq!(stats.games_played(), 1);
        assert!(stats.recent(1).next().unwrap().won);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn streaks_reset_on_loss_and_grow_on_wins() {
        let mut stats = Stats::default();
        stats.push(record(true, 3, 5));
        stats.push(record(true, 2, 5));
        stats.push(record(false, 6, 5));
        stats.push(record(true, 4, 5));

        assert_eq!(stats.current_streak(), 1);
        assert_eq!(stats.max_streak(), 2);
        assert_eq!(stats.win_rate(), Some(75));
    }

    #[test]
    fn aggregates_come_from_wins_only_where_they_should() {
        let mut stats = Stats::default();
        stats.push(record(false, 6, 5));
        stats.push(record(true, 2, 5));
        stats.push(GameRecord {
            timestamp: 2,
            word_size: 5,
            round_limit: 6,
            rounds_played: 4,
            won: true,
            duration_secs: 10,
            answer: "crane".to_owned(),
        });

        let [_, two, _, four, ..] = stats.win_distribution();
        assert_eq!((two, four), (1, 1));
        assert_eq!(stats.average_win_guesses(), Some(3.0));
        assert_eq!(stats.fewest_guesses_win().unwrap().rounds_played, 2);
        assert_eq!(stats.fastest_win().unwrap().duration_secs, 10);
    }

    #[test]
    fn by_word_size_lists_only_played_sizes() {
        let mut stats = Stats::default();
        stats.push(record(true, 3, 5));
        stats.push(record(true, 3, 5));
        stats.push(record(false, 6, 8));

        assert_eq!(stats.by_word_size(), vec![(5, 2, 2), (8, 1, 0)]);
    }

    #[test]
    fn recent_lists_latest_first() {
        let mut stats = Stats::default();
        for timestamp in 1..=6 {
            stats.push(GameRecord {
                timestamp,
                ..record(true, 3, 5)
            });
        }

        let recent: Vec<u64> = stats.recent(RECENT_LIMIT).map(|r| r.timestamp).collect();
        assert_eq!(recent, vec![6, 5, 4, 3, 2]);
    }

    #[test]
    fn empty_stats_have_no_aggregates() {
        let stats = Stats::default();
        assert_eq!(stats.win_rate(), None);
        assert_eq!(stats.average_win_guesses(), None);
        assert_eq!(stats.fewest_guesses_win(), None);
        assert_eq!(stats.fastest_win(), None);
        assert_eq!(stats.current_streak(), 0);
        assert_eq!(stats.max_streak(), 0);
    }
}
