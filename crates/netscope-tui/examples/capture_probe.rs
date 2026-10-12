//! Local measurements only; elapsed times are not regression assertions.
use netscope_import::{Limits, import_bytes};
use netscope_tui::{Action, App, draw};
use ratatui::{Terminal, backend::TestBackend};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected HAR path")?;
    let bytes = std::fs::read(path)?;
    let start = Instant::now();
    let session = import_bytes(&bytes, Limits::default())?;
    let import_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut app = App::new(&session);
    let mut terminal = Terminal::new(TestBackend::new(100, 30))?;
    terminal.draw(|frame| draw(frame, &session, &mut app, "generated.har"))?;
    let start = Instant::now();
    for _ in 0..200 {
        app.apply(Action::Down, &session);
        terminal.draw(|frame| draw(frame, &session, &mut app, "generated.har"))?;
        black_box(terminal.backend());
    }
    let frame_us = start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0;
    println!(
        "{} requests, {} bytes, import_ms={import_ms:.3}, move_draw_us={frame_us:.3}",
        session.requests().len(),
        bytes.len()
    );
    Ok(())
}
