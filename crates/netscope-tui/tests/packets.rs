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
    for (width, height) in [(80, 24), (100, 30)] {
        let mut app = PacketApp::new(&capture);
        for (name, keys) in [
            ("packets", vec![]),
            ("packet-layers", vec![KeyCode::Enter]),
            (
                "stream-evidence",
                vec![
                    KeyCode::Esc,
                    KeyCode::Char('s'),
                    KeyCode::Down,
                    KeyCode::Enter,
                ],
            ),
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
