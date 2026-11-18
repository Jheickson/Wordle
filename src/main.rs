mod app;
mod event;
mod game;
mod ui;
mod words;

use std::io::Result;

use app::App;
use event::handle_events;

fn main() -> Result<()> {
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
