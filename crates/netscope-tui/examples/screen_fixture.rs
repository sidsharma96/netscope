//! Print a fixed in-memory screen for review and golden fixture regeneration.
use netscope_import::{Limits, import_bytes};
use netscope_tui::{Action, App, draw};
use ratatui::{Terminal, backend::TestBackend};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let width: u16 = args.first().map(String::as_str).unwrap_or("100").parse()?;
    let height: u16 = if width == 80 { 24 } else { 30 };
    let mode = args.get(1).map(String::as_str).unwrap_or("overview");
    let bytes: &[u8] = if matches!(mode, "hosts" | "filtered") {
        include_bytes!("../../../tests/fixtures/har/multi-host.har")
    } else if mode == "protocols" {
        include_bytes!("../../../tests/fixtures/har/protocols.har")
    } else if mode == "retries" {
        include_bytes!("../../../tests/fixtures/har/retries.har")
    } else if mode == "redirects" {
        include_bytes!("../../../tests/fixtures/har/redirects.har")
    } else {
        include_bytes!("../../../tests/fixtures/har/single-request.har")
    };
    let session = import_bytes(bytes, Limits::default())?;
    let mut app = App::new(&session);
    app.resize(width, height, &session);
    match mode {
        "draft" | "review" | "draft-invalid" | "running" | "complete" | "cancelled" | "timeout"
        | "limited" | "failed" => {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            app.apply(Action::Open, &session);
            app.apply(Action::Replay, &session);
            if mode != "draft" && mode != "draft-invalid" {
                for _ in 0..4 {
                    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &session);
                }
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
            } else if mode == "draft-invalid" {
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
                app.handle_key(
                    KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
                    &session,
                );
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
            }
            if matches!(
                mode,
                "running" | "complete" | "cancelled" | "timeout" | "limited" | "failed"
            ) {
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
                let editor = app.replay.as_mut().ok_or("missing editor")?;
                let (id, review) = editor.take_send().ok_or("missing send")?;
                if mode != "running" {
                    use netscope_net::{Outcome, ReplayResult};
                    let outcome = match mode {
                        "complete" => Outcome::Complete,
                        "cancelled" => Outcome::Cancelled,
                        "timeout" => Outcome::TimedOut,
                        "limited" => Outcome::SizeLimited,
                        _ => Outcome::Failed("Response body ended with a read error"),
                    };
                    editor.complete(
                        id,
                        ReplayResult {
                            source: review.source,
                            destination: review.destination,
                            outcome,
                            status: Some(200),
                            version: Some("HTTP/1.1".into()),
                            headers: vec![("content-type".into(), b"text/plain".to_vec())],
                            body: b"hello".to_vec(),
                            elapsed: std::time::Duration::from_millis(25),
                        },
                    );
                }
            }
        }
        "redirects" | "retries" | "protocols" => {
            app.apply(Action::Open, &session);
            for _ in 0..1000 {
                app.apply(Action::Down, &session);
            }
        }
        "inspector" => app.apply(Action::Open, &session),
        "hosts" => app.apply(Action::Hosts, &session),
        "filtered" => {
            app.apply(Action::Hosts, &session);
            app.apply(Action::Open, &session);
        }
        _ => {}
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| draw(frame, &session, &mut app, "synthetic.har"))?;
    let buffer = terminal.backend().buffer();
    for y in 0..height {
        println!(
            "{}",
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        );
    }
    Ok(())
}
