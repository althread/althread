use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn check_exit_codes_distinguish_verdicts_and_errors() {
    for (source, max_states, status, message) in [
        ("main { print(1); }", "2", 0, "Verified"),
        (
            "main {} check { eventually false; }",
            "1",
            1,
            "Property violated",
        ),
        (
            "main { print(1); } check { eventually false; }",
            "1",
            4,
            "Verification inconclusive",
        ),
        (
            "shared { let X = 0; } main {} always { 1 / X == 0; }",
            "2",
            3,
            "Cannot divide by zero",
        ),
        ("main {}", "0", 3, "The state limit must be at least 1"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_althread-cli"))
            .args(["check", "--max-states", max_states, "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(status), "{stdout}\n{stderr}");
        assert!(
            stdout.contains(message) || stderr.contains(message),
            "{stdout}\n{stderr}"
        );
    }
}

#[test]
fn run_reports_invariant_violation_with_nonzero_exit_and_correct_interactive_line() {
    let invariant_source = "shared { let X = 0; }\nalways { X == 0; }\nmain { X = 1; }\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_althread-cli"))
        .args(["run", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(invariant_source.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invariant"), "{stderr}");

    let temp_path = std::env::temp_dir().join(format!(
        "althread_cli_interactive_{}.alt",
        std::process::id()
    ));
    std::fs::write(&temp_path, "main {\n    print(42);\n}\n").unwrap();
    let interactive = Command::new(env!("CARGO_BIN_EXE_althread-cli"))
        .args(["run", "--interactive", temp_path.to_str().unwrap()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = interactive.wait_with_output().unwrap();
    let _ = std::fs::remove_file(&temp_path);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("print(42);"),
        "expected interactive preview to show line 2 ('print(42);'), got:\n{stdout}"
    );
}
