//! Controlled render failure for the separate pseudo-terminal restoration check.
use netscope_core::Session;
fn main() -> std::process::ExitCode {
    let session = Session::new(Vec::new());
    let result = netscope_tui::terminal::run_with_renderer(
        &session,
        "probe.har",
        |frame, session, app, filename| {
            netscope_tui::draw(frame, session, app, filename);
            Err(std::io::Error::other("injected rendering error"))
        },
    );
    if result.is_err() {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::SUCCESS
    }
}
