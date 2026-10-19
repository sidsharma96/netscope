//! Fixed packet buffers and offline CLI state checks.
use crossterm::event::{KeyCode, KeyEvent};
use netscope_tui::packet_view::{PacketApp, draw_packets};
use ratatui::{Terminal, backend::TestBackend};
fn render(
    capture: &netscope_core::packets::PacketCapture,
    app: &mut PacketApp,
    width: u16,
    height: u16,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| draw_packets(frame, capture, app, "synthetic.pcap"))?;
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
fn packet_views_and_navigation_preserve_selected_records() -> Result<(), Box<dyn std::error::Error>>
{
    let capture = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/streams.pcap"
    ))?;
    for (width, height) in [(80, 24), (100, 30), (120, 30)] {
        let mut app = PacketApp::new(&capture);
        for (name, keys) in [
            ("packets", vec![]),
            ("packet-explained", vec![KeyCode::Enter]),
            ("packet-layers", vec![KeyCode::Char('2')]),
            (
                "stream-explained",
                vec![
                    KeyCode::Esc,
                    KeyCode::Char('s'),
                    KeyCode::Down,
                    KeyCode::Enter,
                ],
            ),
            ("stream-evidence", vec![KeyCode::Char('2')]),
        ] {
            for code in keys {
                app.key(KeyEvent::from(code), &capture);
            }
            let actual = render(&capture, &mut app, width, height)?;
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../tests/fixtures/terminal/{name}-{width}x{height}.txt"
            ));
            if std::env::var_os("NETSCOPE_UPDATE_PACKET_VIEWS").is_some() {
                std::fs::write(&path, &actual)?;
            }
            assert_eq!(actual, std::fs::read_to_string(path)?);
        }
        assert!(render(&capture, &mut app, 50, 10)?.contains("resize"));
        app.key(KeyEvent::from(KeyCode::Char('p')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("STREAM EVIDENCE"));
        app.key(KeyEvent::from(KeyCode::Esc), &capture);
        let list = render(&capture, &mut app, width, height)?;
        assert!(list.contains("> #1"));
    }
    Ok(())
}
#[test]
fn redirected_packet_cli_and_argument_errors_are_distinct() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/pcap/streams.pcap");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg("--pcap")
        .arg(path)
        .output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    assert!(text.contains("24 packets"));
    assert!(text.contains("Chunked terminator and trailers observed"));
    assert!(text.contains("203.0.113.10"));
    assert!(text.contains("conflicting"));
    let missing = std::process::Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg("--pcap")
        .output()?;
    assert_eq!(missing.status.code(), Some(2));
    let failed = std::process::Command::new(env!("CARGO_BIN_EXE_netscope"))
        .args(["--pcap", "/this/path/is/absent.pcap"])
        .output()?;
    assert_eq!(failed.status.code(), Some(1));
    Ok(())
}

#[test]
fn inspector_bounds_do_not_change_list_selection_and_empty_files_are_safe()
-> Result<(), Box<dyn std::error::Error>> {
    let capture = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/streams.pcap"
    ))?;
    let mut app = PacketApp::new(&capture);
    for _ in 0..20 {
        app.key(KeyEvent::from(KeyCode::Down), &capture);
    }
    app.key(KeyEvent::from(KeyCode::Enter), &capture);
    app.key(KeyEvent::from(KeyCode::Char('2')), &capture);
    app.key(KeyEvent::from(KeyCode::End), &capture);
    assert!(render(&capture, &mut app, 80, 24)?.contains("Answer books.test"));
    app.key(KeyEvent::from(KeyCode::Home), &capture);
    assert!(render(&capture, &mut app, 80, 24)?.contains("Packet #20"));
    app.key(KeyEvent::from(KeyCode::Esc), &capture);
    assert!(render(&capture, &mut app, 80, 24)?.contains("> #20"));
    let empty = netscope_import::pcap::import_pcap(
        &include_bytes!("../../../tests/fixtures/pcap/streams.pcap")[..24],
    )?;
    let mut app = PacketApp::new(&empty);
    app.key(KeyEvent::from(KeyCode::End), &empty);
    app.key(KeyEvent::from(KeyCode::Enter), &empty);
    assert!(render(&empty, &mut app, 80, 24)?.contains("No packet records"));
    app.key(KeyEvent::from(KeyCode::Char('s')), &empty);
    assert!(render(&empty, &mut app, 80, 24)?.contains("0 directed epochs"));
    Ok(())
}
#[test]
fn file_level_packet_notes_are_shown() -> Result<(), Box<dyn std::error::Error>> {
    let sample = include_bytes!("../../../tests/fixtures/pcap/streams.pcap");
    let capture = netscope_import::pcap::import_pcap(&sample[..sample.len() - 1])?;
    let note = "Final packet record truncated; earlier records kept";
    assert!(netscope_tui::packet_view::packet_summary(&capture).contains(note));
    let mut app = PacketApp::new(&capture);
    assert!(render(&capture, &mut app, 100, 30)?.contains("Final packet record truncated"));
    Ok(())
}
#[test]
fn timestamps_show_the_file_resolution() -> Result<(), Box<dyn std::error::Error>> {
    use netscope_tui::packet_view::packet_summary;
    let micro = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/raw-ip.pcap"
    ))?;
    let text = packet_summary(&micro);
    assert!(
        text.contains("recorded timestamp 1700000000.123456\n"),
        "{text}"
    );
    let nano = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/big-nano.pcap"
    ))?;
    assert!(packet_summary(&nano).contains("recorded timestamp 1700000000.123456789\n"));
    Ok(())
}

#[test]
fn explanations_preserve_identity_and_report_access_across_topics_and_resize()
-> Result<(), Box<dyn std::error::Error>> {
    let capture = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/streams.pcap"
    ))?;
    let original = netscope_tui::packet_view::packet_summary(&capture);
    for (width, height) in [(80, 24), (100, 30), (120, 34), (160, 50)] {
        let mut app = PacketApp::new(&capture);
        app.key(KeyEvent::from(KeyCode::Enter), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("[1 Explain]"));
        app.key(KeyEvent::from(KeyCode::Down), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("WHERE DO THESE BYTES"));
        app.key(KeyEvent::from(KeyCode::Down), &capture);
        let ack = render(&capture, &mut app, width, height)?;
        assert!(ack.contains("ACK flag is off"));
        app.key(KeyEvent::from(KeyCode::Char('h')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("FIELD HELP"));
        app.key(KeyEvent::from(KeyCode::Char('h')), &capture);
        assert_eq!(render(&capture, &mut app, width, height)?, ack);
        app.key(KeyEvent::from(KeyCode::PageDown), &capture);
        let _ = render(&capture, &mut app, width, height)?;
        assert!(render(&capture, &mut app, 50, 10)?.contains("resize"));
        let _ = render(&capture, &mut app, width, height)?;
        app.key(KeyEvent::from(KeyCode::Char('2')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("recorded timestamp"));
        app.key(KeyEvent::from(KeyCode::Esc), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("> #0"));
    }
    assert_eq!(
        netscope_tui::packet_view::packet_summary(&capture),
        original
    );
    Ok(())
}

#[test]
fn sequence_wrap_and_non_tcp_absence_stay_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let mut capture = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/streams.pcap"
    ))?;
    let tcp = capture.packets[0].tcp.as_mut().ok_or("missing TCP")?;
    tcp.sequence = u32::MAX;
    tcp.declared_payload = 2;
    let mut app = PacketApp::new(&capture);
    app.key(KeyEvent::from(KeyCode::Enter), &capture);
    app.key(KeyEvent::from(KeyCode::Down), &capture);
    let screen = render(&capture, &mut app, 120, 30)?;
    assert!(screen.contains("starts at 0"));
    assert!(!screen.contains("limit what is readable"));
    app.key(KeyEvent::from(KeyCode::Esc), &capture);
    for _ in 0..20 {
        app.key(KeyEvent::from(KeyCode::Down), &capture);
    }
    app.key(KeyEvent::from(KeyCode::Enter), &capture);
    app.key(KeyEvent::from(KeyCode::Down), &capture);
    let udp = render(&capture, &mut app, 120, 30)?;
    assert!(udp.contains("This packet carries UDP"));
    assert!(!udp.contains("ACK field 0"));
    Ok(())
}

#[test]
fn stream_workbench_links_ranges_bytes_and_explanations() -> Result<(), Box<dyn std::error::Error>>
{
    let capture = netscope_import::pcap::import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/streams.pcap"
    ))?;
    let original = netscope_tui::packet_view::packet_summary(&capture);
    for (width, height) in [(80, 24), (100, 30), (120, 34), (160, 50)] {
        let mut app = PacketApp::new(&capture);
        for code in [
            KeyCode::Char('s'),
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Enter,
        ] {
            app.key(KeyEvent::from(code), &capture);
        }
        let gap = render(&capture, &mut app, width, height)?;
        assert!(gap.contains("MISSING FROM THE CAPTURE"));
        assert!(gap.contains("reading stops at byte 19"), "{gap}");
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/terminal/stream-gap-{width}x{height}.txt"
        ));
        if std::env::var_os("NETSCOPE_UPDATE_PACKET_VIEWS").is_some() {
            std::fs::write(&path, &gap)?;
        }
        assert_eq!(gap, std::fs::read_to_string(path)?);
        assert!(gap.contains("Use ↑↓ to inspect adjacent ranges."));
        app.key(KeyEvent::from(KeyCode::Char('h')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("RANGE HELP"));
        app.key(KeyEvent::from(KeyCode::PageDown), &capture);
        app.key(KeyEvent::from(KeyCode::Char('h')), &capture);
        assert_eq!(render(&capture, &mut app, width, height)?, gap);
        app.key(KeyEvent::from(KeyCode::Char('b')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("?? ??"));
        app.key(KeyEvent::from(KeyCode::End), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("LATER BYTES ARE AVAILABLE"));
        app.key(KeyEvent::from(KeyCode::Char('2')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("Directed epoch #2"));
        app.key(KeyEvent::from(KeyCode::Char('1')), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("LATER BYTES ARE AVAILABLE"));
        assert!(render(&capture, &mut app, 50, 10)?.contains("resize"));
        assert!(render(&capture, &mut app, width, height)?.contains("LATER BYTES ARE AVAILABLE"));
        app.key(KeyEvent::from(KeyCode::Home), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("THE BEGINNING IS READABLE"));
        app.key(KeyEvent::from(KeyCode::Esc), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("> #2"));
        app.key(KeyEvent::from(KeyCode::Down), &capture);
        app.key(KeyEvent::from(KeyCode::Enter), &capture);
        assert!(render(&capture, &mut app, width, height)?.contains("RECORDED COPIES DISAGREE"));
        app.key(KeyEvent::from(KeyCode::Char('b')), &capture);
        let conflict = render(&capture, &mut app, width, height)?;
        assert!(conflict.contains("!!"));
        assert!(conflict.contains("Use ↑ to inspect earlier bytes."));
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/terminal/stream-conflict-{width}x{height}.txt"
        ));
        if std::env::var_os("NETSCOPE_UPDATE_PACKET_VIEWS").is_some() {
            std::fs::write(&path, &conflict)?;
        }
        assert_eq!(conflict, std::fs::read_to_string(path)?);
    }
    assert_eq!(
        netscope_tui::packet_view::packet_summary(&capture),
        original
    );
    Ok(())
}
