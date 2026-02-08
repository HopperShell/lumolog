use std::process::Command;

#[test]
fn test_missing_file_shows_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "nonexistent_file.log"])
        .output()
        .expect("failed to execute");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent_file.log")
            || output.status.code() != Some(0),
        "Should error on missing file"
    );
}

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lumolog") || stdout.contains("USAGE") || stdout.contains("Usage"));
}
