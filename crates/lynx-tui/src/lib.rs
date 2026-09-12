//! Interactive incident-triage dashboard.

mod app;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use lynx_core::{Result, SysPaths};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub use app::{App, Tab};

pub fn run(paths: SysPaths) -> Result<()> {
    enable_raw_mode().map_err(|e| lynx_core::LynxError::Other(e.to_string()))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| lynx_core::LynxError::Other(e.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| lynx_core::LynxError::Other(e.to_string()))?;

    let mut app = App::new(paths);
    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal
            .draw(|f| ui::draw(f, app))
            .map_err(|e| lynx_core::LynxError::Other(e.to_string()))?;

        if event::poll(Duration::from_millis(200))
            .map_err(|e| lynx_core::LynxError::Other(e.to_string()))?
        {
            if let Event::Key(key) =
                event::read().map_err(|e| lynx_core::LynxError::Other(e.to_string()))?
            {
                if key.kind == event::KeyEventKind::Press {
                    app.on_key(key);
                }
            }
        }
        if app.should_quit {
            break;
        }
        app.maybe_refresh();
    }
    Ok(())
}
