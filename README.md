# Wordle

![Rust](https://img.shields.io/badge/Rust-terminal-blue)
![License](https://img.shields.io/badge/license-MIT-blue)

A Wordle clone that lives in the terminal, written in Rust with
[ratatui](https://ratatui.rs). No browser, no dependencies at runtime — the
word lists are embedded in the binary and every game is a single command away.

Play words from 3 to 8 letters, set your own guess limit, and keep a persistent
record of every game in SQLite.

## Features

- **Six word sizes (3–8 letters).** The 5-letter vocabulary is the original
  game's official list; every other size has its own derived list. Picked on
  the menu with ←/→ or `h`/`l`.
- **Adjustable guess limit (1–15, default 6).** Picked on the menu with
  `-`/`+`; the board resizes to fit.
- **Faithful feedback.** Frequency-aware evaluation: an answer letter can mark
  at most as many tiles as it has occurrences, matching the original game.
- **On-screen QWERTY keyboard** under the board, each key showing the best
  state its letter has reached so far.
- **Responsive layout.** Tiles step through 3×1, 5×2, and 7×3 cells as the
  terminal grows; the keyboard scales too, and the menu gains a block-letter
  `WORDLE` title once there's room. Playable from 80×24 up.
- **Persistent stats.** Wins, losses, streaks, guess distribution, opening
  words — stored in SQLite and waiting for you on the next run.

## How to play

Guess the hidden word within the guess limit. Every guess must be a real word
of the game's length; submit it with Enter and each letter tile gets one of
three states:

| Tile | Meaning |
| --- | --- |
| **Correct** (green) | Right letter in the right position |
| **Present** (yellow) | Letter appears elsewhere in the answer |
| **Absent** (dark) | No remaining occurrence of that letter in the answer |

Feedback is frequency-aware: if the answer has one `E` and you guess a word
with two, at most one `E` tile lights up.

A win stops input and offers a new game with `n` or Enter. A loss reveals the
answer. Games abandoned mid-play don't count — only finished games are
recorded.

### Keybindings

`Ctrl-C` quits from any screen. `q` quits from any screen **except** a game in
progress, where it types the letter.

| Screen | Key | Action |
| --- | --- | --- |
| Menu | `↑`/`↓` or `k`/`j` | Select entry (wraps around) |
| Menu | `←`/`→` or `h`/`l` | Word size 3–8, while Start game is hovered |
| Menu | `-`/`_` and `+`/`=` | Guess limit 1–15, while Start game is hovered |
| Menu | `Enter` | Confirm selection |
| Game | letters, `Backspace` | Type and edit the current guess |
| Game | `Enter` | Submit guess |
| Game | `Esc` | Back to menu (game state is kept) |
| Game (finished) | `n` or `Enter` | New game · `q` quits |
| Stats / Credits / Repository | `Esc` or `Enter` | Back to menu |

Rejected guesses tell you why — `Not enough letters`, `Not in word list` —
without consuming a round or clearing the buffer.

## Word lists

One answer list and one valid-guess list per size, embedded at compile time
(`include_str!`), so the game works offline and never touches the filesystem
for words. The answer list of each size is a subset of its valid-guess list.

| Letters | Possible answers | Accepted guesses |
| ---: | ---: | ---: |
| 3 | 304 | 12,977 |
| 4 | 925 | 15,000 |
| 5 | 2,315 | 14,855 |
| 6 | 2,315 | 15,000 |
| 7 | 2,315 | 15,000 |
| 8 | 2,315 | 15,000 |

## Stats

Every finished game is recorded in SQLite at
`$XDG_DATA_HOME/wordle/stats.db` (default `~/.local/share/wordle/stats.db`),
with automatic in-place schema upgrades and one-time import of the legacy text
format. The Stats screen shows:

- played / won / lost / win rate, current and best streak
- guess distribution over wins (scaled bars; losses aren't charted)
- average and fewest guesses on wins, fastest win
- the most played opening word, with its share of games that recorded one
- per-word-size totals and recent games

A missing or unreadable database degrades to in-memory stats for the session —
the app never crashes or overwrites a broken file.

## Install and run

### With Nix

The game is packaged as a Nix flake, so playing needs no Rust toolchain —
just Nix with flakes enabled:

```sh
nix run github:Jheickson/Wordle   # play the latest release
```

From a checkout of the repository:

```sh
nix run .                # play
nix build                # build; the binary lands in result/bin/wordle
nix develop              # dev shell: cargo, clippy, rustfmt, rust-analyzer
```

The flake covers x86_64/aarch64 Linux and macOS.

### From source

Requires a Rust toolchain (edition 2021). SQLite is bundled through
`rusqlite`, so no system SQLite is needed.

```sh
cargo run --release   # play
cargo test            # unit tests
```

## Project structure

| File | Role |
| --- | --- |
| `src/main.rs` | Entry point: terminal setup/teardown, panic safety |
| `src/app.rs` | App state: screen, menu, typed letters, message line |
| `src/game.rs` | Rules: evaluation, board, game state |
| `src/words.rs` | Embedded word lists, guess validation, answer draw |
| `src/stats.rs` | Game records, aggregates, SQLite persistence |
| `src/ui.rs` | Rendering and responsive layout |
| `src/event.rs` | Key handling per screen |

The domain vocabulary lives in [`GLOSSARY.md`](GLOSSARY.md), and the
architectural decisions are recorded in [`docs/adr/`](docs/adr/):

- [ADR 0001 — ratatui modular architecture](docs/adr/0001-ratatui-modular-architecture.md)
- [ADR 0002 — stats record log](docs/adr/0002-stats-record-log.md)
- [ADR 0003 — SQLite storage](docs/adr/0003-sqlite-storage.md)

## Credits

- [Josh Wardle](https://en.wikipedia.org/wiki/Josh_Wardle), creator of
  [Wordle](https://www.nytimes.com/games/wordle/index.html). The 5-letter
  answer and valid-guess lists come from the original game.
- The 3, 4, 6, 7, and 8-letter lists are derived from
  [Peter Norvig](https://norvig.com)'s
  [`count_1w` word frequency list](https://norvig.com/ngrams/count_1w.txt).
- Built on [ratatui](https://ratatui.rs) and
  [crossterm](https://github.com/crossterm-rs/crossterm) for the terminal UI,
  and [rusqlite](https://github.com/rusqlite/rusqlite) for stats persistence.

## License

[MIT](LICENSE)
