//! Print a fixed in-memory screen for review and golden fixture regeneration.
use netscope_import::{Limits, import_bytes};
use netscope_tui::{Action, App, InspectorTab, draw};
use ratatui::{Terminal, backend::TestBackend};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let width: u16 = args.first().map(String::as_str).unwrap_or("100").parse()?;
    let height: u16 = args
        .get(2)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(if width == 80 { 24 } else { 30 });
    let mode = args.get(1).map(String::as_str).unwrap_or("overview");
    let bytes: &[u8] = if matches!(mode, "hosts" | "filtered" | "journey-gaps") {
        include_bytes!("../../../tests/fixtures/har/multi-host.har")
    } else if mode == "sse" {
        include_bytes!("../../../tests/fixtures/har/sse.har")
    } else if mode == "protocols" {
        include_bytes!("../../../tests/fixtures/har/protocols.har")
    } else if mode == "retries" {
        include_bytes!("../../../tests/fixtures/har/retries.har")
    } else if mode == "redirects" {
        include_bytes!("../../../tests/fixtures/har/redirects.har")
    } else {
        include_bytes!("../../../tests/fixtures/har/single-request.har")
    };
    let session = if mode == "saved" {
        netscope_core::Session::new(vec![])
    } else {
        import_bytes(bytes, Limits::default())?
    };
    let mut app = App::new(&session);
    app.resize(width, height, &session);
    match mode {
        "saved" => {
            let saved = netscope_core::SavedDraft::decode(include_bytes!(
                "../../../tests/fixtures/drafts/book.json"
            ))?;
            app = App::from_saved(&session, &saved)?;
        }
        "save-preview" => {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            app.apply(Action::Open, &session);
            app.apply(Action::Replay, &session);
            for c in "wquiet book".chars() {
                app.handle_key(
                    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                    &session,
                );
            }
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
            for c in "book.json".chars() {
                app.handle_key(
                    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                    &session,
                );
            }
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
        }
        "draft" | "review" | "draft-invalid" | "running" | "complete" | "cancelled" | "timeout"
        | "limited" | "failed" | "comparison" | "post-edit" | "post-review" | "post-result" => {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            app.apply(Action::Open, &session);
            app.apply(Action::Replay, &session);
            if mode.starts_with("post-") {
                for code in [KeyCode::Char('m'), KeyCode::Char('t'), KeyCode::Char('b')] {
                    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &session);
                }
                for c in "{\"title\":\"café\"}".chars() {
                    app.handle_key(
                        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                        &session,
                    );
                }
                if mode != "post-edit" {
                    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
                }
            }
            if mode != "draft" && mode != "draft-invalid" && mode != "post-edit" {
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
                "running"
                    | "complete"
                    | "cancelled"
                    | "timeout"
                    | "limited"
                    | "failed"
                    | "comparison"
                    | "post-result"
            ) {
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &session);
                let editor = app.replay.as_mut().ok_or("missing editor")?;
                let (id, review) = editor.take_send().ok_or("missing send")?;
                if mode != "running" {
                    use netscope_net::{Outcome, ReplayResult};
                    let outcome = match mode {
                        "complete" | "comparison" | "post-result" => Outcome::Complete,
                        "cancelled" => Outcome::Cancelled,
                        "timeout" => Outcome::TimedOut,
                        "limited" => Outcome::SizeLimited,
                        _ => Outcome::Failed("Response body ended with a read error"),
                    };
                    editor.complete(
                        id,
                        ReplayResult {
                            method: review.method,
                            request_body_bytes: review.body.len(),
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
            if mode == "comparison" {
                app.handle_key(
                    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
                    &session,
                );
            }
        }
        "redirects" | "retries" | "protocols" => {
            app.apply(Action::Open, &session);
            app.apply(Action::Section(InspectorTab::All), &session);
            for _ in 0..1000 {
                app.apply(Action::Down, &session);
            }
        }
        "journey" => app.apply(Action::Journey, &session),
        "journey-gaps" => {
            app.apply(Action::Down, &session);
            app.apply(Action::Journey, &session);
        }
        "journey-tls" => {
            app.apply(Action::Journey, &session);
            app.apply(Action::Up, &session);
            app.apply(Action::Up, &session);
        }
        "headers" => {
            app.apply(Action::Open, &session);
            app.apply(Action::Section(InspectorTab::Headers), &session);
        }
        "body" => {
            app.apply(Action::Open, &session);
            app.apply(Action::Section(InspectorTab::Body), &session);
        }
        "sse" => {
            app.apply(Action::Open, &session);
            app.apply(Action::Section(InspectorTab::All), &session);
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
    if mode == "sse" {
        for _ in 0..100 {
            let text = (0..width)
                .map(|x| terminal.backend().buffer()[(x, 7)].symbol())
                .collect::<String>();
            if text.contains("SSE EVENTS") {
                break;
            }
            app.apply(Action::Down, &session);
            terminal.draw(|frame| draw(frame, &session, &mut app, "synthetic.har"))?;
        }
    }
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
