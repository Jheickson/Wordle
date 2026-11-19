# ADR 0003: SQLite storage for stats

## Status

Accepted (2026-11-18). Partially supersedes ADR 0002 (its storage format and
location decisions; the record-log and recording rules still hold).

## Context

ADR 0002 stored completed games as a plain-text file, one line per game. The
stats are now a small relational dataset (game records plus derived aggregates)
and the user asked for a real database. The machine has no system SQLite
(`pkg-config`, `libsqlite3`, and the `sqlite3` CLI are all absent), so a
dependency decision was unavoidable.

## Decision

1. **Depend on `rusqlite` 0.43 with the `bundled` feature.** SQLite is compiled
   from source into the binary; no system library, service, or CLI is needed.
   Tradeoff accepted: binary size and cold-build time for zero deployment
   requirements and no further installs.
2. **Schema** lives in `game.db`-style tables under
   `$XDG_DATA_HOME/wordle/stats.db` (fallback `~/.local/share/wordle/stats.db`):
   - `games(id, ended_at, word_size, round_limit, rounds_played, won,
     duration_secs, answer)` with an index on `won`;
   - `meta(key, value)` holding `schema_version`.
   `PRAGMA journal_mode=WAL` is set on open.
3. **Records stay the source of truth.** `Stats` loads every row into memory at
   startup and derives aggregates in Rust, exactly as before; the database is
   append-only persistence. This keeps the stats screen free of query error
   paths and keeps all aggregate behavior unit-testable without a database.
4. **Schema versioning is enforced.** A missing version means a fresh database:
   a legacy `wordle-stats v1` text file next to it is imported in one
   transaction and renamed to `stats.imported` so it cannot be imported twice.
   A version this build does not know is refused, and the app falls back to
   in-memory stats instead of writing to or downgrading a newer database.
   Version 2 adds `games.first_guess` through `ALTER TABLE`; pre-existing rows
   keep a `NULL` opening word and are excluded from opening-word aggregates.
5. **Failure degrades, never crashes.** Unopenable (corrupt, unwritable, newer)
   databases produce `StoreError`; `App::new` converts that to in-memory stats.
   A failed insert during a session keeps the record in memory and loses it at
   exit — the TUI must not die mid-game.

## Consequences

- One new dependency tree (rusqlite + vendored SQLite) and a C compiler at
  build time; the binary grows by roughly 1.3 MB.
- Startup reads the whole history into memory; negligible for a terminal game,
  and it keeps rendering independent of database latency.
- Future aggregates can still be derived in Rust from the loaded records;
  queries can move into SQL later without changing the stored data.
- Hand-edited or partial legacy files import only valid lines; files without
  the expected header are left untouched rather than absorbed.
