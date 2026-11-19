# ADR 0002: Stats as an append-only record log

## Status

Accepted (2026-11-18). Storage format and location superseded by ADR 0003.

## Context

The app tracks completed games (wins, losses, streaks, guess distribution,
per-word-size totals, recent history) and persists them across restarts. The
menu gained a Stats screen. Choices were needed on storage format, location,
dependency budget, and which games count.

## Decision

1. **Store one record per completed game, not precomputed aggregates.** Each
   `GameRecord` holds timestamp, word size, round limit, guesses used, won/lost,
   duration, and the answer. `Stats` derives every aggregate (win rate, streaks,
   win distribution, averages, per-size totals) from the records at read time,
   so stored data and displayed numbers cannot disagree, and new aggregates
   need no migration.
2. **Only completed games are recorded** — a win, or a loss by exhausting the
   round limit. Quitting mid-game records nothing, and a loss resets the streak.
   Recording and saving happen inside `App::submit` the moment the game ends,
   so a crash does not lose the result.
3. **Plain-text file, no new dependencies.** The file starts with the header
   `wordle-stats v1` followed by one `record` line per game
   (`record <timestamp> <word_size> <round_limit> <rounds_played> won|lost
   <duration_secs> <answer>`). A missing file, a wrong header, or an invalid
   line yields defaults (the bad lines are dropped individually) — never a
   crash. The header allows a future format version.
4. **Location follows XDG:** `$XDG_DATA_HOME/wordle/stats`, falling back to
   `~/.local/share/wordle/stats`. `XDG_DATA_HOME` is also the seam tests and
   smoke runs use to avoid touching real user data.
5. **No stats API on the game core.** `src/stats.rs` performs its own file I/O;
   `game.rs` stays pure. `App` owns the `Stats`, records on game end, and saves
   through an optional path (`None` keeps stats in memory for tests).

## Consequences

- The database grows by one short line per game (~50 bytes); re-deriving
  aggregates is trivial at these volumes and needs no cleanup pass.
- Aggregates remain correct even if older app versions wrote the file, as long
  as they used a compatible header.
- A hand-edited or corrupted file degrades to empty/partial stats rather than
  breaking startup.
- Word sizes and round limits with no games are simply absent from the
  per-size breakdown.
