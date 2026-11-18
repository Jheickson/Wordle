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

## 1. Fresh run

1. Run `cargo run`.
2. Expect the alternate screen to open on the main menu: the `WORDLE` title and
   the entries `Start game (5 letters, 6 guesses)`, `Credits`, `Repository`,
   `Quit`, with Start game highlighted, plus the key hint at the bottom.
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
5. Press Down to hover Credits, then Right: expect the Credits entry to stay
   highlighted and no size change. Press Up to return to Start game.
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
4. Press Down to hover Credits, then `+`: expect the Credits entry highlighted
   and no count change. Press Up to return to Start game.
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
