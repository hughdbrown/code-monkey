use std::process::Command;

fn cargo_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["run", "--quiet", "--"]);
    cmd
}

#[test]
fn test_cli_no_subcommand_shows_help() {
    let output = cargo_bin().output().unwrap();
    // clap exits with error when no subcommand is provided
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage") || stderr.contains("code-monkey"),
        "Expected usage info, got: {stderr}"
    );
}

#[test]
fn test_cli_check_validates_script() {
    let output = cargo_bin()
        .args(["check", "examples/demo.cm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("is valid"));
    assert!(stdout.contains("directives"));
    assert!(stdout.contains("action blocks"));
}

#[test]
fn test_cli_check_missing_file_errors() {
    let output = cargo_bin()
        .args(["check", "nonexistent.cm"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_present_dry_run() {
    let output = cargo_bin()
        .args(["present", "--dry-run", "examples/demo.cm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry Run"));
    assert!(stdout.contains("Block"));
}

#[test]
fn test_cli_present_requires_agent_without_dry_run() {
    let output = cargo_bin()
        .args(["present", "examples/demo.cm"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--agent"),
        "Expected --agent requirement, got: {stderr}"
    );
}

#[test]
fn test_cli_present_invalid_agent_address() {
    let output = cargo_bin()
        .args(["present", "--agent", "not-an-address", "examples/demo.cm"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid agent address"),
        "Expected address parse error, got: {stderr}"
    );
}
