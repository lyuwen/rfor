//! TTY detection: when stdout/stderr are not a TTY (e.g. piped), pfor must
//! not emit ANSI escape sequences (no progress bar rendering in pipes).
//!
//! `assert_cmd` runs pfor with piped stdio, so these tests naturally exercise
//! the off-TTY code path.

mod common;
use common::{pfor, ANSI_CSI};

#[test]
fn stdout_has_no_ansi_escapes_when_piped() {
    let out = pfor()
        .args(["echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout.contains(ANSI_CSI),
        "stdout should not contain ANSI escapes when piped, got: {:?}",
        stdout
    );
}

#[test]
fn stderr_has_no_ansi_escapes_when_piped() {
    // Even with parallel jobs (which would show a progress bar on a TTY),
    // stderr must be clean when piped.
    let out = pfor()
        .args(["-j", "2", "echo {}", ":::", "a", "b", "c", "d"])
        .assert()
        .success();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        !stderr.contains(ANSI_CSI),
        "stderr should not contain ANSI escapes when piped, got: {:?}",
        stderr
    );
}

#[test]
fn output_is_plain_text_machine_readable() {
    // Combined stdout from pfor must be parseable as plain text — no
    // carriage returns (\r) from progress bar redraws sneaking in.
    let out = pfor()
        .args(["-j", "4", "echo {}", ":::", "1", "2", "3", "4", "5", "6"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout.contains('\r'),
        "stdout should not contain carriage returns when piped, got bytes: {:?}",
        stdout.as_bytes()
    );
    // Every line should be a clean item
    assert_eq!(stdout.lines().count(), 6);
}

#[test]
fn large_parallel_batch_stays_clean() {
    // Stress test: many parallel jobs, confirm no ANSI leaks.
    let items: Vec<String> = (1..=20).map(|i| i.to_string()).collect();
    let mut args: Vec<&str> = vec!["-j", "8", "echo {}", ":::"];
    args.extend(items.iter().map(|s| s.as_str()));

    let out = pfor().args(args).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();

    assert!(
        !stdout.contains(ANSI_CSI),
        "stdout ANSI leak in large batch"
    );
    assert!(
        !stderr.contains(ANSI_CSI),
        "stderr ANSI leak in large batch"
    );
    assert_eq!(stdout.lines().count(), 20, "expected 20 output lines");
}
