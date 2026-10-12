//! Process-level checks for the offline summary and documented exit codes.
use std::{path::PathBuf, process::Command};
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/har")
        .join(name)
}
#[test]
fn usage_and_misuse_have_distinct_exit_codes() -> Result<(), Box<dyn std::error::Error>> {
    let usage = Command::new(env!("CARGO_BIN_EXE_netscope")).output()?;
    assert!(usage.status.success());
    assert!(String::from_utf8(usage.stdout)?.contains("Usage:"));
    let misuse = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .args(["one", "two"])
        .output()?;
    assert_eq!(misuse.status.code(), Some(2));
    Ok(())
}
#[test]
fn summary_and_empty_capture_are_usable() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg(fixture("stable-order.har"))
        .output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    let rows: Vec<_> = text.lines().filter(|line| line.starts_with('#')).collect();
    assert!(rows[0].starts_with("#1 "));
    assert!(rows[1].starts_with("#2 "));
    assert!(rows[2].starts_with("#0 "));
    assert!(text.contains("Reported total"));
    assert!(text.contains("Summary mode: run in a terminal"));
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg(fixture("empty.har"))
        .output()?;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("No requests recorded"));
    Ok(())
}
#[test]
fn failed_imports_and_hostile_filenames_are_safe() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg(fixture("invalid-required-timing.har"))
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stderr)?;
    assert!(text.contains("log.entries[0].timings.wait"));
    assert!(output.stdout.is_empty());
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg(fixture("missing-\u{1b}[31m.har"))
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stderr)?;
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("\\u{1b}"));
    Ok(())
}

#[test]
fn saved_draft_summary_does_not_expose_body_or_send() -> Result<(), Box<dyn std::error::Error>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/drafts/book.json");
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg("--draft")
        .arg(path)
        .output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    assert!(text.contains("Local book"));
    assert!(text.contains("No request sent"));
    assert!(!text.contains("café"));
    let output = Command::new(env!("CARGO_BIN_EXE_netscope"))
        .arg("--draft")
        .arg(fixture("single-request.har"))
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("Invalid saved-draft"));
    Ok(())
}
