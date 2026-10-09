//! Offline capture viewer, with a summary fallback for redirected output.
use netscope_core::terminal_text;
use netscope_import::{Limits, import_file};
use std::{
    io::{self, IsTerminal},
    path::Path,
    process::ExitCode,
};
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        println!("Usage: netscope <capture.har>");
        return ExitCode::SUCCESS;
    }
    if args.len() != 1 {
        eprintln!("Usage: netscope <capture.har>");
        return ExitCode::from(2);
    }
    let path = Path::new(&args[0]);
    let filename = terminal_text(&path.to_string_lossy());
    match import_file(path, Limits::default()) {
        Err(error) => {
            eprintln!("{filename}: {error}");
            ExitCode::from(1)
        }
        Ok(session) => {
            if io::stdin().is_terminal() && io::stdout().is_terminal() {
                return match netscope_tui::terminal::run(&session, &filename) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(_) => {
                        eprintln!("{filename}: terminal operation failed");
                        ExitCode::from(1)
                    }
                };
            }
            println!(
                "{filename}\n{} requests · {} hosts",
                session.requests().len(),
                session.hosts().len()
            );
            if session.requests().is_empty() {
                println!("No requests recorded");
            }
            for id in session.chronological() {
                if let Some(request) = session.request(*id) {
                    let status = if request.status == 0 {
                        "No HTTP status recorded".into()
                    } else {
                        request.status.to_string()
                    };
                    println!(
                        "#{} {} {} | {} | Reported total: {} ms",
                        id.0,
                        terminal_text(&request.method),
                        terminal_text(&request.url),
                        status,
                        request.total_ms
                    );
                    for warning in &request.warnings {
                        println!("  Warning: {}", terminal_text(warning));
                    }
                }
            }
            println!("Summary mode: run in a terminal for interactive inspection.");
            ExitCode::SUCCESS
        }
    }
}
