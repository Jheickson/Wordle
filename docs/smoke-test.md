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
2. Expect the alternate screen to open with a centered 6×5 board of dimmed
   placeholder squares (one square per remaining guess), an empty message line
   above it, and `Esc: menu` in the bottom-right corner (`q to quit` appears
   there only once the game has ended).
3. Expect no shell text or previous screen contents to show through, and no
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

1. Keep submitting valid five-letter guesses, using the colors to converge on
   the answer (green = right letter in the right place).
2. Once all five tiles in a row are green, expect the message
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
