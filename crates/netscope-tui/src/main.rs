//! Offline capture viewer, with a summary fallback for redirected output.
use netscope_core::terminal_text;
use netscope_import::{Limits, import_file};
use std::{
    io::{self, IsTerminal},
    path::Path,
    process::ExitCode,
};
const HELP: &str = "Netscope — inspect HTTP captures and prepare reviewed requests

Usage:
  netscope <capture.har>
  netscope --pcap <capture.pcap>
  netscope --draft <request.json>
  netscope -- <capture.har>
  netscope --draft -- <request.json>
  netscope --help
  netscope --version

Options:
  -h, --help       Show this help
  -V, --version    Show the installed package version
  --pcap FILE      Inspect classic PCAP packet/stream evidence offline
  --draft FILE     Open a saved request for editing
  --               Treat the following path literally, even if it starts with -

Examples:
  netscope capture.har
  netscope --draft request.json
  netscope -- ./captures/session.har

In a terminal: Enter inspects, / searches URLs, h shows hosts, a clears filters.
Use r to prepare a replay, ? for keys, and q to quit.
Drafts: m changes method, b edits body, w previews a saved file.
Only Review followed by Send starts network traffic. Opening never sends.
Redirected output prints a capture summary or saved-request name.
The interactive view needs at least 80 columns and 24 rows.

Exit status: 0 success, 1 file/validation/terminal failure, 2 invalid arguments.";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("netscope {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if let Some((_, path)) = match args.as_slice() {
        [flag, path] if flag == "--pcap" && !path.to_string_lossy().starts_with('-') => {
            Some((flag, path))
        }
        [flag, separator, path] if flag == "--pcap" && separator == "--" => Some((flag, path)),
        _ => None,
    } {
        let filename = terminal_text(&path.to_string_lossy());
        return match netscope_import::pcap::import_pcap_file(Path::new(path)) {
            Ok(capture) => {
                if io::stdin().is_terminal() && io::stdout().is_terminal() {
                    match netscope_tui::terminal::run_packets(&capture, &filename) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(_) => {
                            eprintln!("{filename}: packet terminal operation failed");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    println!(
                        "{filename}\n{}",
                        netscope_tui::packet_view::packet_summary(&capture)
                    );
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("{filename}: {error}");
                ExitCode::from(1)
            }
        };
    }
    let (draft, path) = match args.as_slice() {
        [flag, separator, path] if flag == "--draft" && separator == "--" => (true, path),
        [flag, path] if flag == "--draft" && !path.to_string_lossy().starts_with('-') => {
            (true, path)
        }
        [separator, path] if separator == "--" => (false, path),
        [path] if !path.to_string_lossy().starts_with('-') => (false, path),
        _ => {
            eprintln!("Invalid arguments. Use netscope --help for usage and examples.");
            return ExitCode::from(2);
        }
    };
    let path = Path::new(path);
    if draft {
        return match netscope_tui::saved_file::load(path) {
            Ok(saved) => {
                if io::stdin().is_terminal() && io::stdout().is_terminal() {
                    match netscope_tui::terminal::run_saved(&saved, &terminal_text(&saved.name)) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(_) => {
                            eprintln!("Terminal operation failed");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    println!(
                        "Saved draft: {}\nRun in a terminal to edit and review. No request sent.",
                        terminal_text(&saved.name)
                    );
                    ExitCode::SUCCESS
                }
            }
            Err(reason) => {
                eprintln!("{reason}");
                ExitCode::from(1)
            }
        };
    }
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
