//! Fixed-buffer and input-state checks without a real terminal or clock.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use netscope_core::{Body, RequestId, Session};
use netscope_import::{Limits, import_bytes};
use netscope_tui::{Action, App, Screen, draw, inspector_lines, terminal::key_action};
use ratatui::{Terminal, backend::TestBackend, style::Color};
fn session() -> Result<Session, Box<dyn std::error::Error>> {
    Ok(import_bytes(
        include_bytes!("../../../tests/fixtures/har/single-request.har"),
        Limits::default(),
    )?)
}
fn render(
    session: &Session,
    app: &mut App,
    width: u16,
    height: u16,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| draw(frame, session, app, "synthetic.har"))?;
    let buffer = terminal.backend().buffer();
    Ok((0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
#[test]
fn navigation_retains_overview_selection_and_scroll() -> Result<(), Box<dyn std::error::Error>> {
    let one = session()?;
    let requests = vec![one.requests()[0].clone(); 50];
    let session = Session::new(requests);
    let mut app = App::new(&session);
    app.resize(80, 24, &session);
    for _ in 0..30 {
        app.apply(Action::Down, &session);
    }
    let saved = (app.selected, app.overview_offset);
    assert_eq!(saved, (Some(30), 16));
    app.apply(Action::Open, &session);
    assert_eq!(app.screen, Screen::Inspector(RequestId(30)));
    for _ in 0..10 {
        app.apply(Action::Down, &session);
    }
    assert_eq!(app.inspector_offset, 10);
    app.apply(Action::Help, &session);
    app.apply(Action::Down, &session);
    assert_eq!(app.inspector_offset, 10);
    app.apply(Action::Back, &session);
    assert!(!app.help);
    assert!(matches!(app.screen, Screen::Inspector(_)));
    app.resize(40, 10, &session);
    app.apply(Action::Down, &session);
    assert_eq!(app.inspector_offset, 10);
    app.resize(80, 24, &session);
    app.apply(Action::Back, &session);
    assert_eq!(app.screen, Screen::Overview);
    assert_eq!((app.selected, app.overview_offset), saved);
    app.apply(Action::Open, &session);
    assert_eq!(app.inspector_offset, 0);
    app.apply(Action::Quit, &session);
    assert!(app.quit);
    Ok(())
}
#[test]
fn empty_and_small_views_have_no_invalid_selection() -> Result<(), Box<dyn std::error::Error>> {
    let empty = Session::new(Vec::new());
    let mut app = App::new(&empty);
    for action in [Action::Up, Action::Down, Action::Open, Action::Back] {
        app.apply(action, &empty);
    }
    assert_eq!(app.selected, None);
    assert_eq!(app.screen, Screen::Overview);
    assert!(render(&empty, &mut app, 80, 24)?.contains("No requests recorded"));
    let small = render(&empty, &mut app, 60, 10)?;
    assert!(small.contains("Terminal too small: use at least 80 columns × 24 rows"));
    app.apply(Action::Quit, &empty);
    assert!(app.quit);
    Ok(())
}
#[test]
fn fixed_overviews_keep_labels_selection_and_bars() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for (width, height) in [(100, 30), (80, 24)] {
        let mut app = App::new(&session);
        let text = render(&session, &mut app, width, height)?;
        for expected in [
            "NETSCOPE / CAPTURE",
            "synthetic.har",
            "1 requests",
            "Reported total",
            "Timeline",
            "> 0",
            "GET",
            "example.test/api/books",
            "200",
            "45.00 ms",
            "━",
            "Enter inspect",
            "q quit",
        ] {
            assert!(text.contains(expected), "Missing {expected}\n{text}");
        }
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|f| draw(f, &session, &mut app, "synthetic.har"))?;
        assert_eq!(
            terminal.backend().buffer()[(1, 6)].bg,
            Color::Rgb(128, 225, 178)
        );
    }
    Ok(())
}
#[test]
fn inspector_scrolls_to_every_section_and_preserves_duplicate_headers()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    let top = render(&session, &mut app, 80, 24)?;
    assert!(top.contains("INSPECTOR"));
    assert!(top.contains("URL: https://example.test/api/books"));
    assert!(top.contains("X-Example: one"));
    assert!(top.contains("X-Example: two"));
    let mut seen = top;
    for _ in 0..100 {
        app.apply(Action::Down, &session);
        seen.push_str(&render(&session, &mut app, 80, 24)?);
    }
    for label in [
        "RESPONSE",
        "Response body",
        "Exported text",
        "[]",
        "TIMINGS",
        "Connect: 30 ms",
        "SSL (within connect): 20 ms",
        "DNS: Unavailable (-1)",
        "Blocked: Not recorded",
        "Waiting for response: 10 ms",
        "WARNINGS",
    ] {
        assert!(seen.contains(label), "{label}");
    }
    let bottom = app.inspector_offset;
    app.apply(Action::Down, &session);
    assert_eq!(app.inspector_offset, bottom);
    Ok(())
}
#[test]
fn representation_labels_and_metadata_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut request = session.requests()[0].clone();
    request.status = 0;
    for (body, label) in [
        (Body::NotRecorded, "Response body not recorded"),
        (Body::Text("".into()), "Captured body is empty"),
        (Body::DecodedText("hi".into()), "Decoded from base64"),
        (Body::Binary(vec![255]), "hexadecimal preview"),
        (
            Body::Undecoded {
                encoded: "xx".into(),
                reason: "malformed base64".into(),
            },
            "Body could not be decoded",
        ),
    ] {
        request.body = body;
        let lines = inspector_lines(&request, RequestId(0)).join("\n");
        assert!(lines.contains(label));
        assert!(lines.contains("No HTTP status recorded"));
        assert!(!lines.contains("Server IP"));
        assert!(!lines.contains("certificate"));
    }
    request.body = Body::Text("a".repeat(20_000));
    assert!(
        inspector_lines(&request, RequestId(0))
            .join("\n")
            .contains("Preview truncated")
    );
    request.connection = Some("".into());
    request.server_ip = Some("::1".parse()?);
    let text = inspector_lines(&request, RequestId(0)).join("\n");
    assert!(text.contains("Connection (exporter-reported)"));
    assert!(text.contains("Server IP (exporter-reported)"));
    Ok(())
}
#[test]
fn unicode_and_hostile_capture_text_wrap_without_controls() -> Result<(), Box<dyn std::error::Error>>
{
    let session = session()?;
    let mut request = session.requests()[0].clone();
    request.headers[0].value = format!("{}\u{1b}[31m\n", "界e\u{301}👩‍💻".repeat(40));
    let session = Session::new(vec![request]);
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    let mut rendered = String::new();
    for _ in 0..30 {
        rendered.push_str(&render(&session, &mut app, 80, 24)?);
        app.apply(Action::Down, &session);
    }
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("\\u{1b}[31m\\n"));
    assert!(rendered.contains("界"));
    assert!(rendered.contains("e\u{301}"));
    assert!(rendered.contains("👩‍💻"));
    Ok(())
}
#[test]
fn keyboard_aliases_and_help_are_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    for (key, action) in [
        (KeyCode::Up, Action::Up),
        (KeyCode::Char('k'), Action::Up),
        (KeyCode::Down, Action::Down),
        (KeyCode::Char('j'), Action::Down),
        (KeyCode::Enter, Action::Open),
        (KeyCode::Esc, Action::Back),
        (KeyCode::Char('?'), Action::Help),
        (KeyCode::Char('q'), Action::Quit),
    ] {
        assert_eq!(
            key_action(KeyEvent::new(key, KeyModifiers::NONE)),
            Some(action)
        );
    }
    let mut release = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert_eq!(key_action(release), None);
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Help, &session);
    assert!(render(&session, &mut app, 100, 30)?.contains("quit and restore terminal"));
    app.apply(Action::Help, &session);
    assert!(!app.help);
    Ok(())
}
#[test]
fn zero_duration_and_status_zero_have_visible_labels() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut request = session.requests()[0].clone();
    request.total_ms = 0.0;
    request.status = 0;
    let session = Session::new(vec![request]);
    let mut app = App::new(&session);
    let text = render(&session, &mut app, 80, 24)?;
    assert!(text.contains("| 0 ms"));
    assert!(text.contains("No HTTP status recorded"));
    Ok(())
}

#[test]
fn reviewed_screen_fixtures_match_complete_buffers() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for (width, height, inspector, expected) in [
        (
            100,
            30,
            false,
            include_str!("../../../tests/fixtures/terminal/overview-100x30.txt"),
        ),
        (
            80,
            24,
            false,
            include_str!("../../../tests/fixtures/terminal/overview-80x24.txt"),
        ),
        (
            80,
            24,
            true,
            include_str!("../../../tests/fixtures/terminal/inspector-80x24.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        if inspector {
            app.apply(Action::Open, &session);
        }
        assert_eq!(
            render(&session, &mut app, width, height)?,
            expected.strip_suffix('\n').unwrap_or(expected)
        );
    }
    Ok(())
}

fn hosts_session() -> Result<Session, Box<dyn std::error::Error>> {
    Ok(import_bytes(
        include_bytes!("../../../tests/fixtures/har/multi-host.har"),
        Limits::default(),
    )?)
}
#[test]
fn host_filter_round_trip_preserves_ids_and_all_request_position()
-> Result<(), Box<dyn std::error::Error>> {
    let session = hosts_session()?;
    let mut app = App::new(&session);
    for _ in 0..4 {
        app.apply(Action::Down, &session);
    }
    let saved = (app.selected, app.overview_offset);
    app.apply(Action::Hosts, &session);
    assert_eq!(app.screen, Screen::Hosts);
    app.apply(Action::Open, &session);
    assert_eq!(app.host_filter, Some(netscope_core::HostId(1)));
    assert_eq!(
        app.request_order(&session),
        [RequestId(1), RequestId(2), RequestId(3)]
    );
    app.apply(Action::Down, &session);
    app.apply(Action::Open, &session);
    assert_eq!(app.screen, Screen::Inspector(RequestId(2)));
    app.apply(Action::Hosts, &session);
    app.apply(Action::Back, &session);
    assert_eq!(app.screen, Screen::Inspector(RequestId(2)));
    app.apply(Action::Back, &session);
    assert_eq!(app.selected, Some(1));
    app.apply(Action::Back, &session);
    assert_eq!(app.screen, Screen::Hosts);
    app.apply(Action::Open, &session);
    assert_eq!(app.selected, Some(1));
    app.apply(Action::Back, &session);
    app.apply(Action::Down, &session);
    app.apply(Action::Open, &session);
    assert_eq!(app.host_filter, Some(netscope_core::HostId(0)));
    assert_eq!(app.selected, Some(0));
    assert_eq!(app.request_order(&session), [RequestId(5), RequestId(0)]);
    app.apply(Action::AllRequests, &session);
    assert!(app.host_filter.is_none());
    assert_eq!((app.selected, app.overview_offset), saved);
    Ok(())
}
#[test]
fn host_navigation_handles_scroll_help_resize_and_empty_input()
-> Result<(), Box<dyn std::error::Error>> {
    let one = session()?;
    let requests = (0..40)
        .map(|i| {
            let mut r = one.requests()[0].clone();
            r.host = format!("host{i:02}.example.test");
            r
        })
        .collect();
    let session = Session::new(requests);
    let mut app = App::new(&session);
    app.resize(80, 24, &session);
    for _ in 0..30 {
        app.apply(Action::Down, &session);
    }
    let saved = (app.selected, app.overview_offset);
    app.apply(Action::Hosts, &session);
    for _ in 0..30 {
        app.apply(Action::Down, &session);
    }
    assert_eq!(app.host_selected, Some(30));
    assert_eq!(app.host_offset, 24);
    app.apply(Action::Help, &session);
    app.apply(Action::Down, &session);
    assert_eq!(app.host_selected, Some(30));
    app.apply(Action::Back, &session);
    app.resize(60, 10, &session);
    app.apply(Action::Open, &session);
    assert_eq!(app.screen, Screen::Hosts);
    app.resize(80, 24, &session);
    app.apply(Action::Open, &session);
    app.apply(Action::AllRequests, &session);
    assert_eq!((app.selected, app.overview_offset), saved);
    let empty = Session::new(vec![]);
    let mut app = App::new(&empty);
    app.apply(Action::Hosts, &empty);
    app.apply(Action::Open, &empty);
    app.apply(Action::Down, &empty);
    assert_eq!(app.host_selected, None);
    assert!(render(&empty, &mut app, 80, 24)?.contains("No hosts recorded"));
    app.resize(30, 10, &empty);
    app.apply(Action::Quit, &empty);
    assert!(app.quit);
    Ok(())
}
#[test]
fn host_buffers_and_filter_labels_match_reviewed_fixtures() -> Result<(), Box<dyn std::error::Error>>
{
    let session = hosts_session()?;
    for (width, height, filtered, expected) in [
        (
            80,
            24,
            false,
            include_str!("../../../tests/fixtures/terminal/hosts-80x24.txt"),
        ),
        (
            100,
            30,
            false,
            include_str!("../../../tests/fixtures/terminal/hosts-100x30.txt"),
        ),
        (
            80,
            24,
            true,
            include_str!("../../../tests/fixtures/terminal/filtered-80x24.txt"),
        ),
        (
            100,
            30,
            true,
            include_str!("../../../tests/fixtures/terminal/filtered-100x30.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        app.apply(Action::Hosts, &session);
        if filtered {
            app.apply(Action::Open, &session);
        }
        let text = render(&session, &mut app, width, height)?;
        assert_eq!(text, expected.strip_suffix('\n').unwrap_or(expected));
        if filtered {
            assert!(text.contains("Host: api.example.test"));
            assert!(text.contains("3 / 6 requests"));
            assert!(text.contains("Row 1 / 3"));
            assert!(!text.contains("assets.example.test"));
        } else {
            assert!(text.contains("No status"));
            assert!(text.contains("120.00"));
            assert!(text.contains("requests can overlap"));
            assert!(text.contains("Requests with import warnings: 1"));
        }
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|frame| draw(frame, &session, &mut app, "synthetic.har"))?;
        assert_eq!(
            terminal.backend().buffer()[(1, 6)].bg,
            Color::Rgb(128, 225, 178)
        );
    }
    Ok(())
}
#[test]
fn hosts_escape_controls_and_keep_long_names_inspectable() -> Result<(), Box<dyn std::error::Error>>
{
    let one = session()?;
    let mut request = one.requests()[0].clone();
    request.host = format!("{}\u{1b}[31m", "界".repeat(90));
    request.total_ms = f64::MAX;
    let session = Session::new(vec![request.clone(), request]);
    let mut app = App::new(&session);
    app.apply(Action::Hosts, &session);
    let text = render(&session, &mut app, 80, 24)?;
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("界"));
    assert!(text.contains("Overflow"));
    assert_eq!(
        key_action(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Some(Action::Hosts)
    );
    assert_eq!(
        key_action(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        Some(Action::AllRequests)
    );
    Ok(())
}

#[test]
fn redirect_evidence_is_visible_after_scrolling_and_preserves_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/redirects.har"),
        Limits::default(),
    )?;
    let evidence = netscope_tui::redirect_lines(&session, RequestId(0)).join("\n");
    assert!(evidence.contains("Ambiguous: multiple candidates"));
    assert!(evidence.contains("Candidate #1"));
    assert!(evidence.contains("Candidate #2"));
    assert!(evidence.contains("0..30 s after reported end"));
    assert!(
        netscope_tui::redirect_lines(&session, RequestId(3))
            .join("\n")
            .contains("One candidate")
    );
    assert!(
        netscope_tui::redirect_lines(&session, RequestId(5))
            .join("\n")
            .contains("No Location header recorded")
    );
    for width in [80, 100] {
        let mut app = App::new(&session);
        let height = if width == 80 { 24 } else { 30 };
        app.resize(width, height, &session);
        app.apply(Action::Open, &session);
        for _ in 0..1000 {
            app.apply(Action::Down, &session);
        }
        let screen = render(&session, &mut app, width, height)?;
        let expected = if width == 80 {
            include_str!("../../../tests/fixtures/terminal/redirects-80x24.txt")
        } else {
            include_str!("../../../tests/fixtures/terminal/redirects-100x30.txt")
        };
        assert_eq!(screen, expected.trim_end_matches('\n'));
        assert!(screen.contains("REDIRECT EVIDENCE"));
        assert!(screen.contains("Candidate #1"));
        assert!(screen.contains("Candidate #2"));
        app.apply(Action::Back, &session);
        assert_eq!(app.selected, Some(0));
    }
    Ok(())
}
