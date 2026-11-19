# Glossary

Domain vocabulary for the terminal Wordle game. Use these terms in issues, tests, and code; don't drift to synonyms.

- **Answer** — the secret word chosen for a game; its length is the game's **word size**. Hidden while playing; revealed on loss.
- **Guess** — a word of the game's **word size** that the player submits. Only words in the **valid-guess list** of that size can be submitted.
- **Evaluation** — the per-position classification of a guess against the answer, producing one **tile state** per letter.
- **Tile state** — one of: **Correct** (letter, right position), **Present** (letter elsewhere in the answer), **Absent** (no remaining occurrence in the answer). Frequency-aware: an answer letter can mark at most as many tiles as it has occurrences in the answer.
- **Board** — the *round limit*×*word size* grid of tiles; row *n* shows the evaluation of guess *n*.
- **Round** — one attempt: type a word, submit, evaluation appears on the board.
- **Round limit** — the maximum number of rounds a game allows, 1 to 15, default 6. Chosen on the menu while Start game is hovered: `-`/`+` (or `_`/`=`) change it; the value clamps at both ends and is shown on the entry.
- **Answer list** — embedded list of possible answers for one word size (`assets/answers-{3..8}.txt`). A superset-restricted subset of the valid-guess list of that size.
- **Valid-guess list** — embedded list of acceptable guesses for one word size (`assets/allowed-{3..8}.txt`); contains the answer list of that size.
- **Game state** — `Playing → Won | Lost`. Won: a guess evaluates all-Correct. Lost: the **round limit** passes without a win.
- **Screen** — which view the TUI shows: **Menu** (startup), **Game**, **Stats**, **Credits**, or **Repository**. `Esc` leaves the game and the info screens for the menu.
- **Menu** — the startup screen's selectable entries: Start game, Stats, Credits, Repository, Quit. The cursor wraps; Start game always begins a fresh game at the selected **word size** and **round limit**.
- **Word size** — the number of letters per word, 3 to 8, default 5. Chosen on the menu while Start game is hovered: Left/Right or `h`/`l` change it; the value clamps at both ends and is shown on the entry.
- **Keyboard** — the QWERTY rows under the board; each key shows the best tile state seen for that letter across played rounds (Correct > Present > Absent). Unguessed keys stay plain.
- **App state** — the TUI model: game state plus the word currently being typed, a transient message line (e.g. "not a valid word"), the screen being shown, the menu cursor, the **stats** with their file path, and the current game's start time.
- **Game record** — one completed game: the end time, the **word size** and **round limit** it used, the guesses it took, whether it was won, its duration, and the **answer**. Only wins and losses are records; a game abandoned mid-play is not.
- **Stats** — every game record plus the aggregates derived from them: games played, wins, losses, win rate, **streak**, **win distribution**, average guesses on wins, fewest-guesses win, fastest win, per-word-size totals, and recent games. Persisted in the **stats file**.
- **Stats file** — `$XDG_DATA_HOME/wordle/stats` (default `~/.local/share/wordle/stats`), a `wordle-stats v1` header plus one `record` line per game. Missing or corrupt contents read as empty stats; invalid lines are ignored.
- **Streak** — consecutive wins ending at the latest game (*current streak*) or the longest such run ever (*best streak*). A loss resets the current streak.
- **Win distribution** — wins counted per number of guesses used; losses are not charted. Shows as scaled bars on the Stats screen.
