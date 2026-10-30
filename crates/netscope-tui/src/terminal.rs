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
use std::panic::PanicHookInfo;
use std::sync::{Mutex, Once, PoisonError};
use std::thread::{self, ThreadId};

type Hook = Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>;
/// The thread that owns the terminal while raw mode or the alternate screen is active.
static UI_THREAD: Mutex<Option<ThreadId>> = Mutex::new(None);
static HOOK: Once = Once::new();

/// Wrap the previous panic hook so a panic on the interface thread first leaves
/// the alternate screen; otherwise the message is printed there and then erased.
fn restoring_hook(previous: Hook, restore: impl Fn() + Send + Sync + 'static) -> Hook {
    Box::new(move |info| {
        let owner = *UI_THREAD.lock().unwrap_or_else(PoisonError::into_inner);
        if owner == Some(thread::current().id()) {
            restore();
        }
        previous(info);
    })
}
fn install_panic_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(restoring_hook(previous, || {
            let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
            let _ = disable_raw_mode();
        }));
    });
}

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
        install_panic_hook();
        *UI_THREAD.lock().unwrap_or_else(PoisonError::into_inner) = Some(thread::current().id());
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
        *UI_THREAD.lock().unwrap_or_else(PoisonError::into_inner) = None;
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
        KeyCode::Tab => Some(Action::Focus),
        KeyCode::Char('g') => Some(Action::Journey),
        KeyCode::Char('1') => Some(Action::Section(crate::InspectorTab::Summary)),
        KeyCode::Char('2') => Some(Action::Section(crate::InspectorTab::Headers)),
        KeyCode::Char('3') => Some(Action::Section(crate::InspectorTab::Body)),
        KeyCode::Char('4') => Some(Action::Section(crate::InspectorTab::Timing)),
        KeyCode::Char('5') => Some(Action::Section(crate::InspectorTab::Evidence)),
        KeyCode::Char('6') => Some(Action::Section(crate::InspectorTab::All)),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Home => Some(Action::First),
        KeyCode::End => Some(Action::Last),
        KeyCode::Enter => Some(Action::Open),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Char('?') => Some(Action::Help),
        KeyCode::Char('h') => Some(Action::Hosts),
        KeyCode::Char('a') => Some(Action::AllRequests),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('/') => Some(Action::Search),
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
    run_app(session, filename, App::new(session), &mut render)
}
/// Open a saved request in the editor with no capture or automatic Send.
/// # Errors
/// Returns terminal setup, event, rendering, or restoration failures.
pub fn run_saved(saved: &netscope_core::SavedDraft, filename: &str) -> io::Result<()> {
    let session = Session::new(vec![]);
    let app = App::from_saved(&session, saved).map_err(io::Error::other)?;
    run_app(
        &session,
        filename,
        app,
        &mut |frame, session, app, filename| {
            draw(frame, session, app, filename);
            Ok(())
        },
    )
}
fn run_app(
    session: &Session,
    filename: &str,
    mut app: App,
    render: &mut impl FnMut(&mut Frame, &Session, &mut App, &str) -> io::Result<()>,
) -> io::Result<()> {
    let mut guard = Restore::enter()?;
    let result = (|| {
        let mut terminal: Terminal<CrosstermBackend<Stdout>> =
            Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut worker: Option<(u64, netscope_net::Worker)> = None;
        let reduced_motion = std::env::var_os("NETSCOPE_REDUCE_MOTION").is_some();
        let mut redraw = true;
        while !app.quit {
            if let Some(editor) = &mut app.replay {
                if let Some((path, saved)) = editor.take_save() {
                    redraw = true;
                    editor.saved(crate::saved_file::save(std::path::Path::new(&path), &saved));
                }
                if let Some((id, active)) = &mut worker {
                    if let Some(snapshot) = active.progress() {
                        redraw = true;
                        editor.progress(*id, snapshot);
                    }
                    // Accept completed work before handling a later cancellation key.
                    if let Some(result) = active.poll() {
                        if let Some(snapshot) = active.progress() {
                            editor.progress(*id, snapshot);
                        }
                        redraw = true;
                        editor.complete(*id, result);
                        worker = None;
                    } else if editor.cancellation_requested() {
                        active.cancel();
                    }
                }
                if worker.is_none()
                    && let Some((id, review)) = editor.take_send()
                {
                    redraw = true;
                    let started = editor
                        .started_at()
                        .ok_or_else(|| io::Error::other("Missing Send time"))?;
                    let operation = if editor.streaming_requested() {
                        netscope_net::Worker::start_stream_at(review, started)
                    } else {
                        netscope_net::Worker::start_at(review, started)
                    };
                    match operation {
                        Ok(active) => worker = Some((id, active)),
                        Err(_) => editor.start_failed(id),
                    }
                }
            }

            if let Some(editor) = &mut app.replay {
                let elapsed = editor
                    .started_at()
                    .map_or(std::time::Duration::ZERO, |at| at.elapsed());
                redraw |= editor.update_activity(elapsed, reduced_motion);
            }
            if redraw {
                let mut rendered = Ok(());
                terminal.draw(|frame| {
                    rendered = render(frame, session, &mut app, filename);
                })?;
                rendered?;
                redraw = false;
            }
            // Static screens sleep until input; an active worker still needs polling.
            if worker.is_some() && !event::poll(std::time::Duration::from_millis(50))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) => {
                    redraw = true;
                    if let (Some(editor), Some((id, active))) = (&mut app.replay, &mut worker) {
                        if let Some(snapshot) = active.progress() {
                            editor.progress(*id, snapshot);
                        }
                        if let Some(result) = active.poll() {
                            if let Some(snapshot) = active.progress() {
                                editor.progress(*id, snapshot);
                            }
                            editor.complete(*id, result);
                            worker = None;
                        }
                    }
                    app.handle_key(key, session);
                }
                Event::Resize(width, height) => {
                    app.resize(width, height, session);
                    redraw = true;
                }
                _ => {}
            }
        }
        Ok(())
    })();
    let restored = guard.restore();
    result.and(restored)
}

/// Run offline packet inspection, restoring the terminal on exits and errors.
/// # Errors
/// Returns terminal setup, drawing, or input errors after attempted restoration.
pub fn run_packets(
    capture: &netscope_core::packets::PacketCapture,
    filename: &str,
) -> io::Result<()> {
    let mut guard = Restore::enter()?;
    let result = (|| {
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut app = crate::packet_view::PacketApp::new(capture);
        while !app.quit() {
            terminal.draw(|frame| {
                crate::packet_view::draw_packets(frame, capture, &mut app, filename)
            })?;
            match event::read()? {
                Event::Key(key) => app.key(key, capture),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    })();
    result.and(guard.restore())
}

#[cfg(test)]
mod tests {
    use super::{UI_THREAD, restoring_hook};
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
    fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
    }
    #[test]
    fn panic_hook_restores_the_interface_thread_before_printing() {
        let events = Arc::new(Mutex::new(vec![]));
        let (printed, restored) = (events.clone(), events.clone());
        let original = std::panic::take_hook();
        std::panic::set_hook(restoring_hook(
            Box::new(move |_| lock(&printed).push("message")),
            move || lock(&restored).push("restore"),
        ));
        *lock(&UI_THREAD) = Some(std::thread::current().id());
        let _ = std::panic::catch_unwind(|| panic!("interface"));
        // A worker panic must not tear down the interface that is still running.
        let _ = std::thread::spawn(|| panic!("worker")).join();
        *lock(&UI_THREAD) = None;
        std::panic::set_hook(original);
        assert_eq!(*lock(&events), ["restore", "message", "message"]);
    }
}
