# Glossary

Domain vocabulary for the terminal Wordle game. Use these terms in issues, tests, and code; don't drift to synonyms.

- **Answer** — the secret 5-letter word chosen for a game. Hidden while playing; revealed on loss.
- **Guess** — a 5-letter word the player submits. Only words in the **valid-guess list** can be submitted.
- **Evaluation** — the per-position classification of a guess against the answer, producing one **tile state** per letter.
- **Tile state** — one of: **Correct** (letter, right position), **Present** (letter elsewhere in the answer), **Absent** (no remaining occurrence in the answer). Frequency-aware: an answer letter can mark at most as many tiles as it has occurrences in the answer.
- **Board** — the 6×5 grid of tiles; row *n* shows the evaluation of guess *n*.
- **Round** — one attempt: type a word, submit, evaluation appears on the board.
- **Answer list** — embedded list of possible answers (`assets/answers.txt`). A superset-restricted subset of the valid-guess list.
- **Valid-guess list** — embedded list of acceptable guesses (`assets/allowed.txt`); contains the answer list.
- **Game state** — `Playing → Won | Lost`. Won: a guess evaluates all-Correct. Lost: 6 rounds pass without a win.
- **Screen** — which view the TUI shows: **Menu** (startup), **Game**, **Credits**, or **Repository**. `Esc` leaves the game and the info screens for the menu.
- **Menu** — the startup screen's selectable entries: Start game, Credits, Repository, Quit. The cursor wraps; Start game always begins a fresh game.
- **Keyboard** — the QWERTY rows under the board; each key shows the best tile state seen for that letter across played rounds (Correct > Present > Absent). Unguessed keys stay plain.
- **App state** — the TUI model: game state plus the word currently being typed, a transient message line (e.g. "not a valid word"), the screen being shown, and the menu cursor.
