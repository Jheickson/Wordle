//! Persistent record of completed games, stored in SQLite.
//!
//! [`GameRecord`] captures one finished game. [`Stats`] keeps every record in
//! memory and derives all aggregates from them, while an optional [`Store`]
//! persists each record to a SQLite database under the XDG data dir.
//!
//! Opening the database creates it, applies the schema (tracked in a `meta`
//! table), and imports a legacy plain-text stats file once. A database that
//! cannot be opened or is newer than this build falls back to in-memory stats:
//! the session keeps working and nothing is corrupted.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::game::{MAX_ROUNDS, MAX_WORD_LEN, MIN_ROUNDS, MIN_WORD_LEN};

/// Schema version this build expects in the `meta` table.
const SCHEMA_VERSION: i64 = 1;

/// File name of the pre-SQLite stats file, next to the database.
const LEGACY_FILE_NAME: &str = "stats";

/// First line of the legacy stats file; only files with it are imported.
const LEGACY_HEADER: &str = "wordle-stats v1";

/// Number of recent games the stats screen lists.
pub const RECENT_LIMIT: usize = 5;

/// Schema statement applied on every open; safe to repeat.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    ended_at INTEGER NOT NULL,
    word_size INTEGER NOT NULL,
    round_limit INTEGER NOT NULL,
    rounds_played INTEGER NOT NULL,
    won INTEGER NOT NULL,
    duration_secs INTEGER NOT NULL,
    answer TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS games_won ON games(won);
";

/// Why a [`Stats`] database could not be opened or upgraded.
#[derive(Debug)]
pub enum StoreError {
    /// Parent directories or the database file could not be reached.
    Io(std::io::Error),
    /// The database rejected the schema, the data, or the legacy import.
    Sqlite(rusqlite::Error),
    /// The database was written by a newer build with an unknown schema.
    UnsupportedSchema(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "stats database I/O failed: {error}"),
            Self::Sqlite(error) => write!(f, "stats database error: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "stats database schema {version} is newer than this build")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

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

/// Every completed game, with a database backing it when one could be opened.
///
/// Aggregates are computed from the in-memory records, so the stats screen
/// never waits on the database once startup has loaded it.
#[derive(Debug, Default)]
pub struct Stats {
    records: Vec<GameRecord>,
    store: Option<Store>,
}

impl Stats {
    /// Opens (creating it if needed) the database at `path` and loads every
    /// stored game, importing a legacy text stats file on first creation.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let store = Store::open(path)?;
        let records = store.load()?;
        Ok(Self {
            records,
            store: Some(store),
        })
    }

    /// Records a completed game, appending it to the database when one is
    /// open. A failed write keeps the game in memory for this session.
    pub fn push(&mut self, record: GameRecord) {
        if let Some(store) = &self.store {
            let _ = store.insert(&record);
        }
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
}

/// The SQLite database behind [`Stats`].
#[derive(Debug)]
struct Store {
    conn: Connection,
}

impl Store {
    /// Opens or creates the database at `path`, applies the schema, and
    /// upgrades or imports as needed.
    fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;
        Self::upgrade(&conn, path)?;
        Ok(Self { conn })
    }

    /// Sets up the schema version, importing a legacy text file on first use.
    fn upgrade(conn: &Connection, path: &Path) -> Result<(), StoreError> {
        let version: Option<String> = conn
            .query_row("SELECT value FROM meta WHERE key = 'schema_version'", [], |row| {
                row.get(0)
            })
            .optional()?;
        match version {
            None => {
                import_legacy(conn, path)?;
                conn.execute(
                    "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)",
                    params![SCHEMA_VERSION.to_string()],
                )?;
                Ok(())
            }
            Some(version) => match version.parse::<i64>() {
                Ok(found) if found == SCHEMA_VERSION => Ok(()),
                _ => Err(StoreError::UnsupportedSchema(version)),
            },
        }
    }

    /// Appends one game.
    fn insert(&self, record: &GameRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO games
                (ended_at, word_size, round_limit, rounds_played, won, duration_secs, answer)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.timestamp as i64,
                record.word_size as i64,
                record.round_limit as i64,
                record.rounds_played as i64,
                record.won as i64,
                record.duration_secs as i64,
                record.answer
            ],
        )?;
        Ok(())
    }

    /// Every stored game, oldest first.
    fn load(&self) -> rusqlite::Result<Vec<GameRecord>> {
        let mut statement = self.conn.prepare(
            "SELECT ended_at, word_size, round_limit, rounds_played, won,
                    duration_secs, answer
             FROM games ORDER BY id",
        )?;
        let records = statement.query_map([], |row| {
            Ok(GameRecord {
                timestamp: row.get::<_, i64>(0)? as u64,
                word_size: row.get::<_, i64>(1)? as usize,
                round_limit: row.get::<_, i64>(2)? as usize,
                rounds_played: row.get::<_, i64>(3)? as usize,
                won: row.get::<_, i64>(4)? != 0,
                duration_secs: row.get::<_, i64>(5)? as u64,
                answer: row.get(6)?,
            })
        })?;
        records.collect()
    }
}

/// Imports a legacy plain-text stats file sitting next to the database, then
/// renames it to `<name>.imported` so it is never imported twice.
///
/// Files without the legacy header are left untouched, so a future format is
/// not silently absorbed.
fn import_legacy(conn: &Connection, db_path: &Path) -> Result<(), StoreError> {
    let legacy = db_path.with_file_name(LEGACY_FILE_NAME);
    let Ok(text) = fs::read_to_string(&legacy) else {
        return Ok(());
    };
    let Some(records) = parse_legacy(&text) else {
        return Ok(());
    };

    let transaction = conn.unchecked_transaction()?;
    for record in records {
        transaction.execute(
            "INSERT INTO games
                (ended_at, word_size, round_limit, rounds_played, won, duration_secs, answer)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.timestamp as i64,
                record.word_size as i64,
                record.round_limit as i64,
                record.rounds_played as i64,
                record.won as i64,
                record.duration_secs as i64,
                record.answer
            ],
        )?;
    }
    transaction.commit()?;
    fs::rename(&legacy, legacy.with_extension("imported"))?;
    Ok(())
}

/// Parses a legacy stats file, or `None` when its header does not match.
/// Invalid `record` lines are dropped individually.
fn parse_legacy(text: &str) -> Option<Vec<GameRecord>> {
    let mut lines = text.lines();
    if lines.next() != Some(LEGACY_HEADER) {
        return None;
    }
    Some(lines.filter_map(parse_legacy_record).collect())
}

/// Parses one legacy `record` line, or `None` when it does not hold a valid
/// game.
fn parse_legacy_record(line: &str) -> Option<GameRecord> {
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

/// Where the database lives: `$XDG_DATA_HOME/wordle/stats.db`, falling back
/// to `~/.local/share/wordle/stats.db`.
pub fn default_path() -> PathBuf {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from(".local/share"));
    data_home.join("wordle").join("stats.db")
}

/// Opens the database at the default path; on failure the caller gets an
/// error it can turn into in-memory stats.
pub fn open_default() -> Result<Stats, StoreError> {
    Stats::open(&default_path())
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

    /// A scratch database in a directory of its own, so the legacy file that
    /// sits next to it cannot collide with another test's.
    fn scratch_db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wordle-stats-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("stats.db")
    }

    fn cleanup(path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = fs::remove_dir_all(dir);
        }
    }

    fn legacy_path(db_path: &Path) -> PathBuf {
        db_path.with_file_name(LEGACY_FILE_NAME)
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
    fn records_survive_a_reopen() {
        let path = scratch_db("reopen");
        let mut stats = Stats::open(&path).unwrap();
        stats.push(record(true, 3, 5));
        stats.push(record(false, 6, 4));
        drop(stats);

        let reopened = Stats::open(&path).unwrap();
        assert_eq!(reopened.games_played(), 2);
        assert_eq!(reopened.wins(), 1);
        assert_eq!(reopened.recent(2).next().unwrap().answer, "aaaa");
        cleanup(&path);
    }

    #[test]
    fn a_fresh_database_imports_the_legacy_text_file_once() {
        let path = scratch_db("legacy");
        let legacy = legacy_path(&path);
        fs::write(
            &legacy,
            format!(
                "{LEGACY_HEADER}\nrecord 1700000000 5 6 3 won 60 crane\nrecord 1 5 6 9 won 60 crane\n"
            ),
        )
        .unwrap();

        let stats = Stats::open(&path).unwrap();
        assert_eq!(stats.games_played(), 1, "invalid lines are dropped");
        drop(stats);
        assert!(!legacy.exists(), "the legacy file is renamed");
        assert!(legacy.with_extension("imported").exists());

        let reopened = Stats::open(&path).unwrap();
        assert_eq!(reopened.games_played(), 1, "no second import");
        let _ = fs::remove_file(legacy.with_extension("imported"));
        cleanup(&path);
    }

    #[test]
    fn a_legacy_file_without_the_header_is_left_alone() {
        let path = scratch_db("no-header");
        let legacy = legacy_path(&path);
        fs::write(&legacy, "garbage\n").unwrap();

        let stats = Stats::open(&path).unwrap();
        assert_eq!(stats.games_played(), 0);
        assert!(legacy.exists());
        let _ = fs::remove_file(&legacy);
        cleanup(&path);
    }

    #[test]
    fn a_corrupt_database_reports_an_error_instead_of_panicking() {
        let path = scratch_db("corrupt");
        fs::write(&path, b"definitely not a sqlite database").unwrap();

        assert!(Stats::open(&path).is_err());
        cleanup(&path);
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_downgraded() {
        let path = scratch_db("newer");
        let stats = Stats::open(&path).unwrap();
        drop(stats);
        let conn = Connection::open(&path).unwrap();
        conn.execute("UPDATE meta SET value = '99' WHERE key = 'schema_version'", [])
            .unwrap();
        drop(conn);

        assert!(matches!(
            Stats::open(&path),
            Err(StoreError::UnsupportedSchema(version)) if version == "99"
        ));
        cleanup(&path);
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
