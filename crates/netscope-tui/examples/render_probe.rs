//! Fixed long-header workloads; timings are observations, never test assertions.
use netscope_core::RequestId;
use netscope_import::{Limits, import_bytes};
use netscope_tui::{App, InspectorTab, Screen, draw};
use ratatui::{Terminal, backend::TestBackend};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let label = std::env::args()
        .nth(1)
        .ok_or("expected a measurement label")?;
    let seed = include_bytes!("../../../tests/fixtures/har/single-request.har");
    println!("label,width,height,header_bytes,sample,frame_ms");
    for (width, height) in [(80, 24), (120, 34)] {
        for size in [4096, 16384, 65536, 262144] {
            let mut data: serde_json::Value = serde_json::from_slice(seed)?;
            data["log"]["entries"][0]["request"]["headers"]
                .as_array_mut()
                .ok_or("headers missing")?
                .push(serde_json::json!({"name": "X-Long", "value": "x".repeat(size)}));
            let session = import_bytes(&serde_json::to_vec(&data)?, Limits::default())?;
            let mut app = App::new(&session);
            app.screen = Screen::Inspector(RequestId(0));
            app.inspector_tab = InspectorTab::Headers;
            let mut terminal = Terminal::new(TestBackend::new(width, height))?;
            for sample in 1..=5 {
                let start = Instant::now();
                terminal.draw(|frame| draw(frame, &session, &mut app, "synthetic.har"))?;
                black_box(terminal.backend());
                println!(
                    "{label},{width},{height},{size},{sample},{:.6}",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }
    }
    Ok(())
}
