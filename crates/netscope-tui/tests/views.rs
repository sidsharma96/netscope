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

#[test]
fn retry_evidence_shows_ambiguity_and_unknowns_without_losing_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/retries.har"),
        Limits::default(),
    )?;
    for (id, phrase) in [
        (0, "Ambiguous:"),
        (3, "One possible retry"),
        (5, "Requires GET or HEAD"),
        (6, "No status recorded"),
        (7, "No candidate under this rule"),
    ] {
        assert!(
            netscope_tui::retry_lines(&session, RequestId(id))
                .join("\n")
                .contains(phrase)
        );
    }
    assert!(netscope_tui::retry_lines(&session, RequestId(1)).is_empty());
    for (width, height, expected) in [
        (
            80,
            24,
            include_str!("../../../tests/fixtures/terminal/retries-80x24.txt"),
        ),
        (
            100,
            30,
            include_str!("../../../tests/fixtures/terminal/retries-100x30.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        app.resize(width, height, &session);
        app.apply(Action::Open, &session);
        for _ in 0..1000 {
            app.apply(Action::Down, &session);
        }
        let screen = render(&session, &mut app, width, height)?;
        assert_eq!(screen, expected.trim_end_matches('\n'));
        assert!(screen.contains("RETRY EVIDENCE"));
        assert!(screen.contains("Candidate #2"));
        app.apply(Action::Back, &session);
        assert_eq!(app.selected, Some(0));
    }
    Ok(())
}

#[test]
fn protocol_notes_preserve_labels_and_separate_unknown_from_disagreement()
-> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/protocols.har"),
        Limits::default(),
    )?;
    let labels = [
        ("HTTP/2", "h2"),
        ("HTTP/1.1", "HTTP/3"),
        ("HTTP/4", "h3"),
        ("HTTP/1.0", "HTTP/1.0"),
        ("HTTP/3.0", "h3"),
        ("unknown", "unknown"),
    ];
    for (r, (sent, received)) in session.requests().iter().zip(labels) {
        assert_eq!(r.request_version, sent);
        assert_eq!(r.response_version, received);
        let lines = netscope_tui::protocol_lines(r).join("\n");
        assert!(lines.contains(&format!("Request label: {sent:?}")));
        assert!(lines.contains(&format!("Response label: {received:?}")));
        assert!(lines.contains("not observed messages"));
        assert!(lines.contains("No compression savings or connection reuse are established"));
    }
    for (id, expected) in [
        (0, "Labels agree"),
        (1, "Labels disagree"),
        (2, "Comparison unavailable"),
        (3, "Labels agree"),
        (4, "Labels agree"),
        (5, "Comparison unavailable"),
    ] {
        assert!(
            netscope_tui::protocol_lines(&session.requests()[id])
                .join("\n")
                .contains(expected)
        );
    }
    let mut hostile = session.requests()[0].clone();
    hostile.request_version = "HTTP/2\x1b[2J\n".into();
    let text = netscope_tui::protocol_lines(&hostile).join("\n");
    assert!(!text.contains('\x1b'));
    assert!(text.contains("Unrecognized label"));
    assert_eq!(hostile.request_version, "HTTP/2\x1b[2J\n");
    Ok(())
}

#[test]
fn protocol_notes_are_reachable_in_fixed_terminal_buffers() -> Result<(), Box<dyn std::error::Error>>
{
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/protocols.har"),
        Limits::default(),
    )?;
    for (width, height, expected) in [
        (
            80,
            24,
            include_str!("../../../tests/fixtures/terminal/protocols-80x24.txt"),
        ),
        (
            100,
            30,
            include_str!("../../../tests/fixtures/terminal/protocols-100x30.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        app.resize(width, height, &session);
        app.apply(Action::Open, &session);
        for _ in 0..1000 {
            app.apply(Action::Down, &session);
        }
        let screen = render(&session, &mut app, width, height)?;
        assert_eq!(screen, expected.trim_end_matches('\n'));
        assert!(screen.contains("PROTOCOL NOTES"));
        assert!(screen.contains("Labels agree"));
        app.apply(Action::Back, &session);
        assert_eq!(app.selected, Some(0));
    }
    Ok(())
}

fn press(app: &mut App, session: &Session, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), session);
}
#[test]
fn replay_edits_are_local_and_text_keys_do_not_quit() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let original = session.requests()[0].clone();
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Down, &session);
    let offset = app.inspector_offset;
    press(&mut app, &session, KeyCode::Char('r'));
    assert_eq!(app.screen, Screen::Replay(RequestId(0)));
    press(&mut app, &session, KeyCode::Down);
    press(&mut app, &session, KeyCode::Enter);
    assert!(
        app.replay
            .as_ref()
            .ok_or("missing draft")?
            .draft()
            .selected(0)
    );
    press(&mut app, &session, KeyCode::Up);
    press(&mut app, &session, KeyCode::Enter);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    for c in "http://localhost:8080/q?x=é".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    assert!(!app.quit);
    press(&mut app, &session, KeyCode::Backspace);
    press(&mut app, &session, KeyCode::Enter);
    assert!(
        !app.replay
            .as_ref()
            .ok_or("missing draft")?
            .draft()
            .selected(0)
    );
    assert_eq!(
        app.replay.as_ref().ok_or("missing draft")?.draft().url(),
        "http://localhost:8080/q?x="
    );
    assert_eq!(session.requests()[0], original);
    press(&mut app, &session, KeyCode::Esc);
    assert_eq!(app.screen, Screen::Inspector(RequestId(0)));
    assert_eq!(app.inspector_offset, offset);
    assert!(app.replay.is_none());
    Ok(())
}
#[test]
fn replay_review_is_masked_and_invalidated_by_editing() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Replay, &session);
    press(&mut app, &session, KeyCode::Down);
    press(&mut app, &session, KeyCode::Enter);
    for _ in 0..3 {
        press(&mut app, &session, KeyCode::Down);
    }
    press(&mut app, &session, KeyCode::Enter);
    let review = render(&session, &mut app, 100, 30)?;
    assert!(review.contains("Effective destination:"));
    assert!(review.contains("[hidden; v reveals all values]"));
    assert!(review.contains("Enter or s sends"));
    assert!(!app.quit);
    assert_eq!(app.screen, Screen::Replay(RequestId(0)));
    press(&mut app, &session, KeyCode::Char('v'));
    assert!(render(&session, &mut app, 100, 30)?.contains("X-Example: one"));
    press(&mut app, &session, KeyCode::Esc);
    for _ in 0..4 {
        press(&mut app, &session, KeyCode::Up);
    }
    press(&mut app, &session, KeyCode::Enter);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    press(&mut app, &session, KeyCode::Enter);
    for _ in 0..4 {
        press(&mut app, &session, KeyCode::Down);
    }
    press(&mut app, &session, KeyCode::Enter);
    let invalid = render(&session, &mut app, 80, 24)?;
    assert!(!invalid.contains("Effective destination:"));
    assert!(invalid.contains("Use an absolute HTTP(S)"));
    Ok(())
}
#[test]
fn replay_manual_headers_cancel_resize_and_ineligible_source()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Replay, &session);
    for _ in 0..3 {
        press(&mut app, &session, KeyCode::Down);
    }
    press(&mut app, &session, KeyCode::Enter);
    for c in "X-New".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    for c in "q?hajk".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    assert!(!app.quit);
    assert_eq!(
        app.replay.as_ref().ok_or("missing draft")?.draft().added()[0].value,
        "q?hajk"
    );
    app.resize(50, 10, &session);
    press(&mut app, &session, KeyCode::Esc);
    assert!(app.replay.is_some());
    app.resize(80, 24, &session);
    press(&mut app, &session, KeyCode::Enter);
    assert!(
        app.replay
            .as_ref()
            .ok_or("missing draft")?
            .draft()
            .added()
            .is_empty()
    );
    press(&mut app, &session, KeyCode::Esc);
    let mut post = session.requests()[0].clone();
    post.method = "DELETE".into();
    let session = Session::new(vec![post]);
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Replay, &session);
    assert!(app.replay.is_none());
    assert!(render(&session, &mut app, 100, 30)?.contains("require GET or POST"));
    Ok(())
}

#[test]
fn replay_draft_and_review_buffers_match_at_both_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for (width, height, review, expected) in [
        (
            80,
            24,
            false,
            include_str!("../../../tests/fixtures/terminal/draft-80x24.txt"),
        ),
        (
            100,
            30,
            false,
            include_str!("../../../tests/fixtures/terminal/draft-100x30.txt"),
        ),
        (
            80,
            24,
            true,
            include_str!("../../../tests/fixtures/terminal/review-80x24.txt"),
        ),
        (
            100,
            30,
            true,
            include_str!("../../../tests/fixtures/terminal/review-100x30.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        app.resize(width, height, &session);
        app.apply(Action::Open, &session);
        app.apply(Action::Replay, &session);
        if review {
            for _ in 0..4 {
                press(&mut app, &session, KeyCode::Down);
            }
            press(&mut app, &session, KeyCode::Enter);
        }
        assert_eq!(
            render(&session, &mut app, width, height)?,
            expected.trim_end_matches('\n')
        );
    }
    Ok(())
}

#[test]
fn replay_invalid_buffers_and_edit_cancellation_are_stable()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for (width, height, expected) in [
        (
            80,
            24,
            include_str!("../../../tests/fixtures/terminal/draft-invalid-80x24.txt"),
        ),
        (
            100,
            30,
            include_str!("../../../tests/fixtures/terminal/draft-invalid-100x30.txt"),
        ),
    ] {
        let mut app = App::new(&session);
        app.resize(width, height, &session);
        app.apply(Action::Open, &session);
        app.apply(Action::Replay, &session);
        press(&mut app, &session, KeyCode::Enter);
        press(&mut app, &session, KeyCode::Char('q'));
        press(&mut app, &session, KeyCode::Esc);
        assert_eq!(
            app.replay.as_ref().ok_or("missing draft")?.draft().url(),
            session.requests()[0].url
        );
        press(&mut app, &session, KeyCode::Enter);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &session,
        );
        press(&mut app, &session, KeyCode::Enter);
        assert_eq!(
            render(&session, &mut app, width, height)?,
            expected.trim_end_matches('\n')
        );
        press(&mut app, &session, KeyCode::Enter);
        app.resize(50, 10, &session);
        press(&mut app, &session, KeyCode::Char('q'));
        assert!(!app.quit);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &session,
        );
        assert!(app.quit);
    }
    Ok(())
}

#[test]
fn invalid_manual_header_name_remains_editable() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Replay, &session);
    for _ in 0..3 {
        press(&mut app, &session, KeyCode::Down);
    }
    press(&mut app, &session, KeyCode::Enter);
    for c in "bad name".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    let screen = render(&session, &mut app, 80, 24)?;
    assert!(screen.contains("NEW HEADER NAME"));
    assert!(screen.contains("bad name"));
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    for c in "X-Fixed".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    press(&mut app, &session, KeyCode::Enter);
    assert_eq!(
        app.replay.as_ref().ok_or("missing draft")?.draft().added()[0].name,
        "X-Fixed"
    );
    Ok(())
}

fn new_replay_result(outcome: netscope_net::Outcome) -> netscope_net::ReplayResult {
    netscope_net::ReplayResult {
        method: netscope_core::ReplayMethod::Get,
        request_body_bytes: 0,
        source: RequestId(0),
        destination: "http://localhost/example".into(),
        outcome,
        status: Some(200),
        version: Some("HTTP/1.1".into()),
        headers: vec![("x-example".into(), b"secret\x1b[31m".to_vec())],
        body: b"hello\x1b[31m".to_vec(),
        elapsed: std::time::Duration::from_millis(25),
    }
}
fn reviewed_app(session: &Session) -> App {
    let mut app = App::new(session);
    app.apply(Action::Open, session);
    app.apply(Action::Replay, session);
    for _ in 0..4 {
        press(&mut app, session, KeyCode::Down);
    }
    press(&mut app, session, KeyCode::Enter);
    app
}
#[test]
fn send_is_explicit_unique_and_accepts_only_one_matching_result()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = reviewed_app(&session);
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    press(&mut app, &session, KeyCode::Char('s'));
    let (id, _) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    press(&mut app, &session, KeyCode::Char('s'));
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id + 1, new_replay_result(netscope_net::Outcome::Complete));
    assert!(render(&session, &mut app, 100, 30)?.contains("Sending a new request"));
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id, new_replay_result(netscope_net::Outcome::Complete));
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id, new_replay_result(netscope_net::Outcome::Cancelled));
    let result = render(&session, &mut app, 100, 30)?;
    assert!(result.contains("Outcome: Complete"));
    assert!(!result.contains("secret"));
    assert!(!result.contains('\x1b'));
    press(&mut app, &session, KeyCode::Esc);
    press(&mut app, &session, KeyCode::Char('o'));
    assert!(render(&session, &mut app, 100, 30)?.contains("Outcome: Complete"));
    Ok(())
}
#[test]
fn cancel_waits_for_cleanup_and_preserves_a_separate_result()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = reviewed_app(&session);
    press(&mut app, &session, KeyCode::Enter);
    let (id, _) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    press(&mut app, &session, KeyCode::Esc);
    assert!(
        app.replay
            .as_ref()
            .ok_or("editor")?
            .cancellation_requested()
    );
    assert!(render(&session, &mut app, 100, 30)?.contains("Cancelling"));
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id, new_replay_result(netscope_net::Outcome::Cancelled));
    assert!(render(&session, &mut app, 100, 30)?.contains("o opens its retained result"));
    press(&mut app, &session, KeyCode::Char('o'));
    assert!(render(&session, &mut app, 100, 30)?.contains("Outcome: Cancelled"));
    assert_eq!(session.requests()[0].url, "https://example.test/api/books");
    Ok(())
}

#[test]
fn running_and_each_result_match_fixed_buffers() -> Result<(), Box<dyn std::error::Error>> {
    use netscope_net::{Outcome, ReplayResult};
    let session = session()?;
    for mode in [
        "running",
        "complete",
        "cancelled",
        "timeout",
        "limited",
        "failed",
    ] {
        for (width, height) in [(80, 24), (100, 30)] {
            let mut app = reviewed_app(&session);
            press(&mut app, &session, KeyCode::Enter);
            let editor = app.replay.as_mut().ok_or("editor")?;
            let (id, review) = editor.take_send().ok_or("send")?;
            if mode != "running" {
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
                        method: netscope_core::ReplayMethod::Get,
                        request_body_bytes: 0,
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
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../tests/fixtures/terminal/{mode}-{width}x{height}.txt"
            ));
            assert_eq!(
                render(&session, &mut app, width, height)?,
                std::fs::read_to_string(path)?.trim_end_matches('\n'),
                "{mode} at {width}"
            );
        }
    }
    Ok(())
}

#[test]
fn comparison_navigation_preserves_result_capture_and_masking()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let original = session.requests()[0].clone();
    let mut app = reviewed_app(&session);
    press(&mut app, &session, KeyCode::Enter);
    let (id, _) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id, new_replay_result(netscope_net::Outcome::Complete));
    press(&mut app, &session, KeyCode::Char('v'));
    press(&mut app, &session, KeyCode::Char('c'));
    let first = render(&session, &mut app, 80, 24)?;
    assert!(first.contains("RESPONSE COMPARISON"));
    assert!(first.contains("Different stored URL text"));
    for _ in 0..100 {
        press(&mut app, &session, KeyCode::Down);
    }
    let bottom = render(&session, &mut app, 80, 24)?;
    assert!(bottom.contains("[hidden; v reveals values]"));
    assert!(!bottom.contains("secret"));
    press(&mut app, &session, KeyCode::Char('v'));
    assert!(render(&session, &mut app, 100, 30)?.contains("secret"));
    assert!(!render(&session, &mut app, 100, 30)?.contains('\x1b'));
    app.resize(40, 10, &session);
    assert!(render(&session, &mut app, 40, 10)?.contains("80"));
    app.resize(100, 30, &session);
    press(&mut app, &session, KeyCode::Esc);
    let returned = render(&session, &mut app, 100, 30)?;
    assert!(returned.contains("NEW RESULT"));
    assert!(!returned.contains("secret"));
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    assert_eq!(session.requests()[0], original);
    Ok(())
}
#[test]
fn comparison_buffers_match_at_both_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for (width, height) in [(80, 24), (100, 30)] {
        let mut app = reviewed_app(&session);
        press(&mut app, &session, KeyCode::Enter);
        let editor = app.replay.as_mut().ok_or("editor")?;
        let (id, review) = editor.take_send().ok_or("send")?;
        editor.complete(
            id,
            netscope_net::ReplayResult {
                method: netscope_core::ReplayMethod::Get,
                request_body_bytes: 0,
                source: review.source,
                destination: review.destination,
                outcome: netscope_net::Outcome::Complete,
                status: Some(200),
                version: Some("HTTP/1.1".into()),
                headers: vec![("content-type".into(), b"text/plain".to_vec())],
                body: b"hello".to_vec(),
                elapsed: std::time::Duration::from_millis(25),
            },
        );
        press(&mut app, &session, KeyCode::Char('c'));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/terminal/comparison-{width}x{height}.txt"
        ));
        assert_eq!(
            render(&session, &mut app, width, height)?,
            std::fs::read_to_string(path)?.trim_end_matches('\n')
        );
    }
    Ok(())
}

#[test]
fn comparison_keeps_the_sent_destination_after_later_draft_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = reviewed_app(&session);
    press(&mut app, &session, KeyCode::Enter);
    let (id, _) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    app.replay
        .as_mut()
        .ok_or("editor")?
        .complete(id, new_replay_result(netscope_net::Outcome::Complete));
    press(&mut app, &session, KeyCode::Esc);
    for _ in 0..4 {
        press(&mut app, &session, KeyCode::Up);
    }
    press(&mut app, &session, KeyCode::Enter);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    for c in "http://localhost/unsent-edit".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    press(&mut app, &session, KeyCode::Char('o'));
    press(&mut app, &session, KeyCode::Char('c'));
    let screen = render(&session, &mut app, 100, 30)?;
    assert!(screen.contains("http://localhost/example"));
    assert!(!screen.contains("unsent-edit"));
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    Ok(())
}
#[test]
fn comparison_preserves_unknown_status_partial_data_and_unicode_limits()
-> Result<(), Box<dyn std::error::Error>> {
    let one = session()?;
    let mut captured = one.requests()[0].clone();
    captured.status = 0;
    captured.body = Body::Text("界🙂".repeat(300));
    let session = Session::new(vec![captured]);
    for status in [None, Some(200)] {
        let mut app = reviewed_app(&session);
        press(&mut app, &session, KeyCode::Enter);
        let (id, _) = app
            .replay
            .as_mut()
            .ok_or("editor")?
            .take_send()
            .ok_or("send")?;
        let mut result = new_replay_result(netscope_net::Outcome::TimedOut);
        result.status = status;
        result.body = "界🙂".repeat(250).into_bytes();
        if status.is_none() {
            result.version = None;
            result.headers.clear();
            result.body.clear();
        }
        app.replay.as_mut().ok_or("editor")?.complete(id, result);
        press(&mut app, &session, KeyCode::Char('c'));
        let first = render(&session, &mut app, 100, 30)?;
        assert!(first.contains("Status: unavailable"));
        assert!(first.contains(if status.is_some() {
            "Partial replay:"
        } else {
            "Status: no response headers"
        }));
        for _ in 0..1000 {
            press(&mut app, &session, KeyCode::Down);
        }
        let bottom = render(&session, &mut app, 80, 24)?;
        assert!(bottom.contains("preview shortened"));
        assert!(!bottom.contains('\x1b'));
        assert!(!bottom.contains('�'));
    }
    Ok(())
}

#[test]
fn comparison_reveals_non_utf8_header_bytes_as_hex() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = reviewed_app(&session);
    press(&mut app, &session, KeyCode::Enter);
    let (id, _) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    let mut result = new_replay_result(netscope_net::Outcome::Complete);
    result.headers = vec![("x-raw".into(), vec![0xff, 0x1b])];
    app.replay.as_mut().ok_or("editor")?.complete(id, result);
    press(&mut app, &session, KeyCode::Char('c'));
    press(&mut app, &session, KeyCode::Char('v'));
    for _ in 0..100 {
        press(&mut app, &session, KeyCode::Down);
    }
    let screen = render(&session, &mut app, 100, 30)?;
    assert!(screen.contains("ff 1b [hex bytes]"));
    assert!(!screen.contains('�'));
    assert!(!screen.contains('\x1b'));
    Ok(())
}

fn post_app(session: &Session, body: &str, editing: bool) -> App {
    let mut app = App::new(session);
    app.apply(Action::Open, session);
    app.apply(Action::Replay, session);
    for code in [KeyCode::Char('m'), KeyCode::Char('t'), KeyCode::Char('b')] {
        press(&mut app, session, code);
    }
    for c in body.chars() {
        press(&mut app, session, KeyCode::Char(c));
    }
    if !editing {
        press(&mut app, session, KeyCode::Enter);
        for _ in 0..4 {
            press(&mut app, session, KeyCode::Down);
        }
        press(&mut app, session, KeyCode::Enter);
    }
    app
}
#[test]
fn body_editing_cancels_locally_and_preserves_text_shortcuts()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = post_app(&session, "qmtb🙂", true);
    press(&mut app, &session, KeyCode::Backspace);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        &session,
    );
    press(&mut app, &session, KeyCode::Tab);
    assert!(!app.quit);
    press(&mut app, &session, KeyCode::Enter);
    assert_eq!(
        app.replay.as_ref().ok_or("editor")?.draft().body(),
        "qmtb\n\t"
    );
    press(&mut app, &session, KeyCode::Char('b'));
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    press(&mut app, &session, KeyCode::Esc);
    assert_eq!(
        app.replay.as_ref().ok_or("editor")?.draft().body(),
        "qmtb\n\t"
    );
    press(&mut app, &session, KeyCode::Char('m'));
    assert!(
        app.replay
            .as_ref()
            .ok_or("editor")?
            .draft()
            .body()
            .is_empty()
    );
    assert_eq!(session.requests()[0].method, "GET");
    Ok(())
}
#[test]
fn invalid_json_cannot_send_and_the_result_retains_the_reviewed_method()
-> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = post_app(&session, "{", false);
    press(&mut app, &session, KeyCode::Char('s'));
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    assert!(render(&session, &mut app, 100, 30)?.contains("Body must be one valid JSON"));
    press(&mut app, &session, KeyCode::Char('b'));
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &session,
    );
    for c in "{\"title\":\"café\"}".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    press(&mut app, &session, KeyCode::Enter);
    assert!(render(&session, &mut app, 100, 30)?.contains("Content-Length: 17 UTF-8 bytes"));
    press(&mut app, &session, KeyCode::Char('s'));
    let (id, review) = app
        .replay
        .as_mut()
        .ok_or("editor")?
        .take_send()
        .ok_or("send")?;
    assert_eq!(review.method, netscope_core::ReplayMethod::Post);
    assert_eq!(review.body, "{\"title\":\"café\"}");
    press(&mut app, &session, KeyCode::Char('s'));
    assert!(app.replay.as_mut().ok_or("editor")?.take_send().is_none());
    let mut result = new_replay_result(netscope_net::Outcome::Complete);
    result.method = review.method;
    result.request_body_bytes = review.body.len();
    app.replay.as_mut().ok_or("editor")?.complete(id, result);
    press(&mut app, &session, KeyCode::Esc);
    press(&mut app, &session, KeyCode::Char('m'));
    press(&mut app, &session, KeyCode::Char('o'));
    assert!(
        render(&session, &mut app, 100, 30)?.contains("POST · HTTP/1.1 · 17 request body bytes")
    );
    press(&mut app, &session, KeyCode::Char('c'));
    let comparison = render(&session, &mut app, 100, 30)?;
    assert!(comparison.contains("Captured method: GET"));
    assert!(comparison.contains("Reviewed: POST; 17 body bytes"));
    Ok(())
}
#[test]
fn post_edit_review_and_result_buffers_match() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    for mode in ["post-edit", "post-review", "post-result"] {
        for (width, height) in [(80, 24), (100, 30)] {
            let mut app = post_app(&session, "{\"title\":\"café\"}", mode == "post-edit");
            if mode == "post-result" {
                press(&mut app, &session, KeyCode::Enter);
                let editor = app.replay.as_mut().ok_or("editor")?;
                let (id, review) = editor.take_send().ok_or("send")?;
                editor.complete(
                    id,
                    netscope_net::ReplayResult {
                        method: review.method,
                        request_body_bytes: review.body.len(),
                        source: review.source,
                        destination: review.destination,
                        outcome: netscope_net::Outcome::Complete,
                        status: Some(200),
                        version: Some("HTTP/1.1".into()),
                        headers: vec![("content-type".into(), b"text/plain".to_vec())],
                        body: b"hello".to_vec(),
                        elapsed: std::time::Duration::from_millis(25),
                    },
                );
            }
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../tests/fixtures/terminal/{mode}-{width}x{height}.txt"
            ));
            assert_eq!(
                render(&session, &mut app, width, height)?,
                std::fs::read_to_string(path)?.trim_end_matches('\n')
            );
        }
    }
    Ok(())
}

#[test]
fn save_preview_never_sends_and_loading_has_no_capture() -> Result<(), Box<dyn std::error::Error>> {
    let session = session()?;
    let mut app = App::new(&session);
    app.apply(Action::Open, &session);
    app.apply(Action::Replay, &session);
    press(&mut app, &session, KeyCode::Char('w'));
    for c in "quiet book".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    assert!(!app.quit);
    press(&mut app, &session, KeyCode::Enter);
    for c in "book.json".chars() {
        press(&mut app, &session, KeyCode::Char(c));
    }
    press(&mut app, &session, KeyCode::Enter);
    let screen = render(&session, &mut app, 100, 30)?;
    assert!(screen.contains("SAVE TO: book.json"));
    assert!(screen.contains("Plain-text file"));
    let editor = app.replay.as_mut().ok_or("missing editor")?;
    assert!(editor.take_send().is_none());
    assert!(editor.take_save().is_none());
    press(&mut app, &session, KeyCode::Enter);
    let editor = app.replay.as_mut().ok_or("missing editor")?;
    let (path, saved) = editor.take_save().ok_or("save not queued")?;
    assert_eq!(path, "book.json");
    assert!(editor.take_save().is_none());
    assert!(editor.take_send().is_none());
    let empty = Session::new(vec![]);
    let mut reopened = App::from_saved(&empty, &saved)?;
    let screen = render(&empty, &mut reopened, 100, 30)?;
    assert!(screen.contains("Saved draft · no capture"));
    assert!(!screen.contains("Source #"));
    assert!(
        reopened
            .replay
            .as_mut()
            .ok_or("editor")?
            .take_send()
            .is_none()
    );
    // A reopened draft has only Edit URL, Add header, and Review rows.
    press(&mut reopened, &empty, KeyCode::Down);
    press(&mut reopened, &empty, KeyCode::Down);
    press(&mut reopened, &empty, KeyCode::Enter);
    assert!(
        reopened
            .replay
            .as_mut()
            .ok_or("editor")?
            .take_send()
            .is_none()
    );
    press(&mut reopened, &empty, KeyCode::Enter);
    let editor = reopened.replay.as_mut().ok_or("editor")?;
    let (id, review) = editor.take_send().ok_or("missing explicit send")?;
    editor.complete(
        id,
        netscope_net::ReplayResult {
            method: review.method,
            request_body_bytes: review.body.len(),
            source: review.source,
            destination: review.destination,
            outcome: netscope_net::Outcome::Complete,
            status: Some(200),
            version: Some("HTTP/1.1".into()),
            headers: vec![],
            body: vec![],
            elapsed: std::time::Duration::ZERO,
        },
    );
    press(&mut reopened, &empty, KeyCode::Char('c'));
    let screen = render(&empty, &mut reopened, 100, 30)?;
    assert!(screen.contains("NEW RESULT"));
    assert!(!screen.contains("RESPONSE COMPARISON"));
    press(&mut reopened, &empty, KeyCode::Esc);
    press(&mut reopened, &empty, KeyCode::Char('w'));
    press(&mut reopened, &empty, KeyCode::Esc);
    assert!(
        reopened
            .replay
            .as_mut()
            .ok_or("editor")?
            .take_save()
            .is_none()
    );
    press(&mut reopened, &empty, KeyCode::Esc);
    assert!(reopened.quit);
    Ok(())
}

#[test]
fn saved_and_save_preview_buffers_match() -> Result<(), Box<dyn std::error::Error>> {
    for (width, height, preview_expected, saved_expected) in [
        (
            80,
            24,
            include_str!("../../../tests/fixtures/terminal/save-preview-80x24.txt"),
            include_str!("../../../tests/fixtures/terminal/saved-80x24.txt"),
        ),
        (
            100,
            30,
            include_str!("../../../tests/fixtures/terminal/save-preview-100x30.txt"),
            include_str!("../../../tests/fixtures/terminal/saved-100x30.txt"),
        ),
    ] {
        let session = session()?;
        let mut app = App::new(&session);
        app.apply(Action::Open, &session);
        app.apply(Action::Replay, &session);
        for c in "wquiet book".chars() {
            press(&mut app, &session, KeyCode::Char(c));
        }
        press(&mut app, &session, KeyCode::Enter);
        for c in "book.json".chars() {
            press(&mut app, &session, KeyCode::Char(c));
        }
        press(&mut app, &session, KeyCode::Enter);
        assert_eq!(
            render(&session, &mut app, width, height)?,
            preview_expected.trim_end_matches('\n')
        );
        let saved = netscope_core::SavedDraft::decode(include_bytes!(
            "../../../tests/fixtures/drafts/book.json"
        ))?;
        let empty = Session::new(vec![]);
        let mut app = App::from_saved(&empty, &saved)?;
        assert_eq!(
            render(&empty, &mut app, width, height)?,
            saved_expected.trim_end_matches('\n')
        );
    }
    Ok(())
}
