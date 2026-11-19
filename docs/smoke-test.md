# Manual smoke test

Step-by-step procedure for checking the terminal Wordle MVP by hand (or with a
pty driver). Every step states the observable result; stop and record a failure
if you see anything else.

## Preconditions

- Run from the repository root, in a real terminal (not a dumb pipe): the app
  uses the alternate screen, raw mode, and colors.
- `cargo build` succeeds.
- Unless a step says otherwise, start the app with `cargo run` and quit it before
  moving to the next section.
- Stats persist in the SQLite database `$XDG_DATA_HOME/wordle/stats.db`
  (default `~/.local/share/wordle/stats.db`). For a clean slate, remove that
  database (and its `-wal`/`-shm` files) or run with `XDG_DATA_HOME` pointing
  at a scratch directory, e.g. `XDG_DATA_HOME=/tmp/wordle-smoke cargo run`.

## 1. Fresh run

1. Run `cargo run`.
2. Expect the alternate screen to open on the main menu: the `WORDLE` title and
   the entries `Start game (5 letters, 6 guesses)`, `Stats`, `Credits`,
   `Repository`, `Quit`, with Start game highlighted, plus the key hint at the
   bottom.
3. Press Enter: expect a centered 6×5 board of dimmed placeholder squares (one
   square per remaining guess), an empty message line above it, and `Esc: menu`
   in the bottom-right corner (`q to quit` appears there only once the game has
   ended).
4. Expect no shell text or previous screen contents to show through, and no
   cursor blinking over the board (the app hides it while drawing).

## 2. Invalid guesses are rejected

1. Type `zzzzz` and press Enter.
2. Expect the message `Not in word list` on the message line, the board still
   empty, and no round consumed (the next valid guess still occupies row 1).
3. A rejected word stays in the input buffer so it can be corrected: press
   Backspace five times, then type `abc` and press Enter.
4. Expect the message `Not enough letters`, the board still empty, and the
   typed letters `abc` still editable.
5. Press Backspace three times: expect the typed letters to disappear one by
   one and the message line to clear on the first edit.

## 3. Valid guess and feedback

1. Type `crane` and press Enter.
2. Expect row 1 to show `C R A N E` in colored tiles: green for exact matches,
   yellow for letters present elsewhere, dark/gray for absent letters.
3. Expect the typed-letters area to be empty again so row 2 can be typed.

## 4. Win

1. Keep submitting valid guesses, using the colors to converge on the
   answer (green = right letter in the right place).
2. Once all tiles in a row are green, expect the message
   `You won! press n for a new game` and the game to stop accepting letters.
3. If six guesses are used up first, restart from section 1 and try again.

Optional deterministic variant: temporarily replace the body of `App::new` in
`src/app.rs` with
`Self::with_answer(Answer::new("crane").expect("valid word"))`, run the app, and
guess `crane`. Revert the edit afterwards (`git checkout src/app.rs`).

## 5. New game from the won state

1. From the won state of section 4, press `n`.
2. Expect a board showing only dimmed placeholder squares again, the message
   line cleared, and play resumed.
3. Win again and press Enter instead of `n`: expect the same fresh game.

## 6. Loss reveals the answer

1. Submit the same valid word six times. Pick a word that does not win and keep
   the feedback off all-green (e.g. repeat a guess that showed a non-green
   tile each round).
2. On the sixth submission, expect the message line to read
   `The word was <ANSWER>; press n for a new game`, with the answer in capital
   letters, and the full board to stay visible.
3. Check that the revealed answer is consistent with at least one green tile
   from your guesses, if you had any.

## 7. New game from the lost state

1. From the lost state of section 6, press `n`.
2. Expect a fresh board of placeholder squares and a cleared message line.
3. Lose again (repeat section 6) and press Enter instead of `n`: expect the same
   fresh game.

## 8. Quit with `q`

1. Start a game, then press `q` while playing: expect the letter `q` in the
   active row and the app still running. Finish or lose the game, then press
   `q`: expect the app to quit.
2. Expect the alternate screen to close, the shell prompt to return, the cursor
   to be visible and blinking, and the shell to be fully usable (type a command,
   press Enter, see it run).
3. Expect the process exit status to be 0 (`echo $?` right after the app exits).
4. Expect no leftover alternate-screen contents or color changes in the shell.

## 9. Quit with Ctrl-C

1. Run `cargo run` again.
2. Press Ctrl-C.
3. Expect the same clean exit as section 8: alternate screen closed, cursor
   visible, shell usable, exit status 0. Neither a `c` nor a `C` letter may
   appear in the board: raw mode delivers Ctrl-C to the app, which treats it as
   quit rather than input.
4. Run again and type a plain `c` (no control key): expect it to appear as a
   typed letter in the active row, proving plain `c` still types.

## 10. Panic leaves a usable terminal

1. Temporarily add `panic!("terminal safety smoke test");` as the first line of
   `run` in `src/main.rs`.
2. Run `cargo run`.
3. Expect the alternate screen to close immediately, the panic message
   `terminal safety smoke test` plus a source location and backtrace note to be
   readable on the normal screen, the cursor to be visible, and no stray colors
   or raw-mode behavior.
4. Run a shell command: it must behave normally (typed characters echoed,
   Enter executes).
5. Expect a non-zero exit status (`echo $?` reports `101`).
6. Remove the temporary `panic!` line and confirm the tree is clean
   (`git status`) and that `cargo build` still succeeds without warnings.

## 11. Word size selection

1. Run `cargo run` and stop on the menu. Expect `Start game (5 letters, 6
   guesses)` as the highlighted first entry and the hint `↑/↓ select · ←/→
   letters · -/+ guesses · Enter confirm · q quit`.
2. Press Right twice: expect the entry to read `Start game (7 letters, 6
   guesses)` and the app to stay on the menu. Press Left once: expect `Start
   game (6 letters, 6 guesses)`.
3. Press Left repeatedly: the size stops at `Start game (3 letters, 6 guesses)`
   and does not wrap below it.
4. Press Right repeatedly: the size stops at `Start game (8 letters, 6
   guesses)` and does not wrap above it.
5. Press Down twice to hover Credits, then Right: expect the Credits entry to
   stay highlighted and no size change. Press Up twice to return to Start game.
6. Select 4 letters and press Enter: expect a centered 4-wide, 6-row board of
   dimmed placeholders (a 4-letter game).
7. Press Enter with an empty buffer: expect `Not enough letters`. Type `zqxj`
   and press Enter: expect `Not in word list`.
8. Submit six valid four-letter guesses (e.g. `that`, `this`, `with`, `from`,
   `have`, `more`): each must be accepted onto the board, and after the sixth
   expect either `You won! press n for a new game` or `The word was <ANSWER>;
   press n for a new game`.
9. Press `n`: expect a fresh 4-wide, 6-row board.
10. Press Esc, then select 8 letters and press Enter: expect a centered 8-wide
    board. Type eight letters (`zqxjzqxj`) and press Enter: expect `Not in word
    list` after accepting all eight characters, proving the input buffer tracks
    the selected size.
11. Press Esc: expect the menu entry to still read `Start game (8 letters, 6
    guesses)`. Press `q`: expect the app to quit cleanly (exit status 0).

## 12. Guess limit selection

1. Run `cargo run` and stop on the menu with Start game hovered. Press `+`
   three times: expect `Start game (5 letters, 9 guesses)`. Press `-` once:
   expect `Start game (5 letters, 8 guesses)`.
2. Press `+` repeatedly: the count stops at `Start game (5 letters, 15
   guesses)` and does not wrap above it.
3. Press `-` repeatedly: the count stops at `Start game (5 letters, 1 guess)`
   and does not wrap below it.
4. Press Down twice to hover Credits, then `+`: expect the Credits entry
   highlighted and no count change. Press Up twice to return to Start game.
5. Set 2 guesses and press Enter: expect a 2-row, 5-wide board. Submit valid
   guesses until the message line shows `You won! press n for a new game` or
   `The word was <ANSWER>; press n for a new game`; it must appear after at
   most two submissions, and no third guess is accepted as a round.
6. Press `n`: expect a fresh 2-row board (the limit is kept).
7. Press Esc, select 15 guesses, and press Enter: expect a 15-row board that
   still fits the 80×24 screen (row gaps collapse), with the keyboard and the
   `Esc: menu` hint visible below it. Submit a guess and expect it on row 1.
8. Submit fifteen valid five-letter guesses (e.g. `crane`, `slate`, `audio`,
   `house`, `plant`, `stone`, `brown`, `clock`, `drink`, `glory`, `honey`,
   `ideal`, `joint`, `knife`, `lemon`): after the fifteenth expect the end-state
   message and the last row's letters to be visible on screen (no clipped-away
   row at 80×24).
9. Press Esc: expect `Start game (5 letters, 15 guesses)`. Press `q`: expect a
   clean exit (status 0).

## 13. Terminal-size responsiveness

Component sizes derive from the terminal: tiles step through 3×1, 5×2, and 7×3
cells (never smaller, never larger), keyboard keys grow with the leftover
height up to 3 rows, and the menu shows a block-letter `WORDLE` title once the
screen has room for it. The board and its components stay centered.

1. Run `cargo run` in an 80×24 terminal and start a 5-letter game: expect a
   board of five 5×2 tiles per row with 1-cell gaps, 3-wide keyboard keys, and
   no overlap.
2. Resize the terminal to roughly 120×40 without quitting: expect the tiles to
   grow to 7×3 and the keyboard keys to grow to 7 wide by 3 tall; the layout
   fills the new space, nothing overlaps, and no stale cells remain.
3. Resize to roughly 200×50: expect the tiles to stay at the 7×3 cap with even
   margins, and the menu (press Esc) to show the block-letter `WORDLE` title at
   double size with the entries as spaced selection bars.
4. Resize back to 80×24: expect the compact layout again, still playable.
5. Set 8 letters and 15 guesses on the menu and enter the game at 80×24:
   expect 3×1 tiles with the row gaps collapsed, the keyboard below the board,
   and `Esc: menu` visible; a submitted guess must land on row 1.

## 14. Stats

1. Start from a clean stats database (see Preconditions). Open the menu, press
   Down once: expect the `Stats` entry highlighted, between Start game and
   Credits. Press Enter: expect a `STATS` screen reading
   `No games played yet: finish one to fill this in.` and the hint
   `Esc: back · q quit`. Press Esc: expect the menu.
2. Start a 3-letter game and submit six distinct valid guesses from
   `assets/allowed-3.txt` (e.g. `aaa`, `aab`, `aac`, `aad`, `aae`, `aaf`).
   Finish on a win or a loss. Expect `stats.db` (plus `stats.db-wal`/`-shm`) to
   exist, its `games` table to gain exactly one row for the game, and
   `meta.schema_version` to read `2`.
3. Press Esc, Down, Enter: expect `Played 1 · Won <0|1> · Lost <1|0> · Win rate
   <0%|100%>`, `Streak`/`Best` values matching the outcome, a `By word size`
   line reading `3 letters · 1 played · …`, and one `Recent games` line. A loss
   line reads `LOST · the word was <ANSWER> · 3 letters · just now`; a win line
   reads `WON in <n> · 3 letters · just now` and the `Guess distribution (wins)`
   section appears above `By word size`.
4. Quit with `q`, restart, and open Stats again: expect the same numbers — the
   history survives a relaunch.
5. Lose another game, then open Stats: expect `Played 2`, `Best` unchanged, and
   `Streak 0`. Win two games in a row: expect `Streak 2`.
6. Replace the database with garbage (`echo garbage >` its path) and restart:
   expect the app to open normally and the Stats screen to show the
   `No games played yet` placeholder instead of crashing; the garbage file must
   still be unchanged after quitting (the app never overwrites an unusable
   database).
7. Open a fresh 3-letter game, submit `aaa` first, and finish it; then start
   another game, submit `aaa` first again, and finish it. Open Stats: expect a
   `Most played opening word AAA (2 games · 100%)` line above `Average
   guesses`. Repeat a third game opening with `aab` and finish it: expect
   `AAA (2 games · 67%)`. Games imported from the legacy text file must not
   count toward that percentage.
8. In an 80×12 terminal, open Stats with a populated history: expect the
   headline and the guess distribution to fit without horizontal clipping, and
   lower sections (`By word size`, `Recent games`) to be dropped whole rather
   than half-drawn.
