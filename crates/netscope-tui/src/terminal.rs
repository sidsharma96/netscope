//! Terminal effects are isolated from navigation and drawing tests.
use crate::{Action, App, draw};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use netscope_core::Session;
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

struct Restore {
    raw: bool,
    alternate: bool,
}
impl Restore {
    fn enter() -> io::Result<Self> {
        let mut guard = Self {
            raw: false,
            alternate: false,
        };
        enable_raw_mode()?;
        guard.raw = true;
        // Mark first so a partially failed terminal write still triggers cleanup.
        guard.alternate = true;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
    fn restore(&mut self) -> io::Result<()> {
        let screen = if self.alternate {
            execute!(io::stdout(), Show, LeaveAlternateScreen)
        } else {
            Ok(())
        };
        let raw = if self.raw { disable_raw_mode() } else { Ok(()) };
        if screen.is_ok() {
            self.alternate = false;
        }
        if raw.is_ok() {
            self.raw = false;
        }
        screen.and(raw)
    }
}
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
/// Convert press/repeat events to actions; release events do not move twice.
pub fn key_action(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::Enter => Some(Action::Open),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Char('?') => Some(Action::Help),
        KeyCode::Char('h') => Some(Action::Hosts),
        KeyCode::Char('a') => Some(Action::AllRequests),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('r') => Some(Action::Replay),
        _ => None,
    }
}
/// Run the interactive viewer, restoring terminal state after success or error.
/// # Errors
/// Returns terminal setup, drawing, event-read, or restoration failures.
pub fn run(session: &Session, filename: &str) -> io::Result<()> {
    run_with_renderer(session, filename, |frame, session, app, filename| {
        draw(frame, session, app, filename);
        Ok(())
    })
}
/// Run with a supplied renderer, allowing controlled rendering-error checks.
/// # Errors
/// Returns the renderer's error after restoring the terminal, or an I/O failure.
pub fn run_with_renderer(
    session: &Session,
    filename: &str,
    mut render: impl FnMut(&mut Frame, &Session, &mut App, &str) -> io::Result<()>,
) -> io::Result<()> {
    let mut guard = Restore::enter()?;
    let result = (|| {
        let mut terminal: Terminal<CrosstermBackend<Stdout>> =
            Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut app = App::new(session);
        while !app.quit {
            let mut rendered = Ok(());
            terminal.draw(|frame| {
                rendered = render(frame, session, &mut app, filename);
            })?;
            rendered?;
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key, session);
                }
                Event::Resize(width, height) => app.resize(width, height, session),
                _ => {}
            }
        }
        Ok(())
    })();
    let restored = guard.restore();
    result.and(restored)
}
