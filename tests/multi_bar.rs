//! Tests for `--multi-bar` flag: per-job progress bars.
//!
//! Sprint 4 feature. With `--multi-bar`, each active job gets its own
//! progress indicator instead of a single shared bar.
//!
//! Since assert_cmd runs with piped stdio (non-TTY), we can't verify
//! visual bar rendering. We test that the flag is accepted, doesn't crash,
//! and doesn't leak ANSI to piped output.

mod common;
use common::{rfor, ANSI_CSI};

// ─── 1. Flag is accepted ─────────────────────────────────────────────

#[test]
fn multi_bar_flag_accepted() {
    // `rfor --multi-bar -j 2 'echo {}' ::: a b` runs without error.
    let out = rfor()
        .args(["--multi-bar", "-j", "2", "echo {}", ":::", "a", "b"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["a", "b"]);
}

// ─── 2. Default (no --multi-bar) unchanged ────────────────────────────

#[test]
fn default_single_bar_unchanged() {
    // Without --multi-bar, behavior is the same as before.
    let out = rfor()
        .args(["-j", "2", "echo {}", ":::", "x", "y"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["x", "y"]);
}

// ─── 3. --multi-bar with -j 1: no crash ──────────────────────────────

#[test]
fn multi_bar_with_sequential_no_crash() {
    // -j 1 + --multi-bar shouldn't crash (graceful single-worker case).
    let out = rfor()
        .args(["--multi-bar", "-j", "1", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["a", "b", "c"]);
}

// ─── 4. --multi-bar + --dry-run ───────────────────────────────────────

#[test]
fn multi_bar_with_dry_run() {
    // --multi-bar + --dry-run: dry-run still prints commands, no conflict.
    let out = rfor()
        .args(["--multi-bar", "--dry-run", "-j", "2", "echo {}", ":::", "a", "b"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.lines().count(), 2, "dry-run should print 2 commands");
    for line in stdout.lines() {
        assert!(line.contains("echo"), "dry-run line should contain 'echo'");
    }
}

// ─── 5. --multi-bar + --group ─────────────────────────────────────────

#[test]
fn multi_bar_with_group() {
    // --multi-bar + --group: both flags accepted, output correct.
    let out = rfor()
        .args([
            "--multi-bar", "--group", "-j", "2",
            "echo {}", ":::", "a", "b", "c",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["a", "b", "c"]);
}

// ─── 6. Non-TTY + --multi-bar: no ANSI artifacts ─────────────────────

#[test]
fn multi_bar_no_ansi_when_piped() {
    // When piped (non-TTY), --multi-bar should not leak ANSI escapes.
    let out = rfor()
        .args([
            "--multi-bar", "-j", "4",
            "echo {}", ":::", "a", "b", "c", "d",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        !stdout.contains(ANSI_CSI),
        "stdout should not contain ANSI escapes when piped with --multi-bar"
    );
    assert!(
        !stderr.contains(ANSI_CSI),
        "stderr should not contain ANSI escapes when piped with --multi-bar"
    );
}
