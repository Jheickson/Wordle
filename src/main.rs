mod app;
mod event;
mod game;
mod stats;
mod ui;
mod words;

use std::io::Result;

use app::App;
use event::handle_events;

fn main() -> Result<()> {
    // Terminal safety on panic is delegated to `ratatui::init`: it installs a
    // panic hook that restores the terminal (disables raw mode and leaves the
    // alternate screen) before chaining to the hook it replaced, so the panic
    // message prints on a usable terminal and the exit code stays non-zero.
    // Unwinding then drops `Terminal`, which shows the cursor again. Installing
    // our own hook before this call would be redundant: `init` would simply
    // chain to it after already restoring the terminal.
    let mut terminal = ratatui::init();
    let app_result = run(&mut terminal);
    ratatui::restore();
    app_result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;
        if handle_events(&mut app)? {
            break;
        }
    }
    Ok(())
}
