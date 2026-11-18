# ADR 0001: Ratatui TUI with a pure game core

## Status

Accepted (2026-11-18)

## Context

The repo contains a working Ratatui skeleton (`ratatui` 0.29 + `crossterm` 0.28) with four modules: `main.rs` (init/restore + draw-and-handle loop), `app.rs` (state), `event.rs` (input), `ui.rs` (render). No game logic exists yet. We need an MVP playable Wordle that stays testable and simple.

## Decision

1. Keep the existing four-module TUI split:
   - `main.rs` — owns the terminal lifecycle (`ratatui::init`/`restore`) and the run loop.
   - `app.rs` — the mutable model the loop threads through events and rendering.
   - `event.rs` — reads crossterm events and translates them into model updates.
   - `ui.rs` — pure rendering of the model; no input handling, no game rules.
2. Add a pure game core (`src/game.rs`) that owns **evaluation**, game state transitions, and guess validity. It MUST NOT depend on `ratatui` or `crossterm` (or any I/O), so it is unit-testable in isolation. `app.rs` composes it; `ui.rs`/`event.rs` only read/drive it through `app.rs`.
3. Word lists are text assets (`assets/answers.txt`, `assets/allowed.txt`) embedded at compile time via `include_str!` — no runtime file or network dependency; swapping lists never touches game logic.
4. MVP rendering is board-only (6×5 grid + message line). No on-screen keyboard; that can be added later without touching the core.

## Consequences

- Game rules (including the repeated-letter case) are covered by unit tests without a terminal.
- UI churn (colors, layout, keyboard row) cannot break the rules.
- Binary carries the word lists (~100 KB); acceptable for a terminal game.
- `event.rs` handles raw keys only; all word-level decisions (validity, evaluation, win/loss) live in the game core.
