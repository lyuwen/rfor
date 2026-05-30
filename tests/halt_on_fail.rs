//! `--halt-on-fail`: stops dispatching new jobs on first failure.
//!
//! Strategy: use sentinel files to prove that jobs after the failing one
//! did NOT run. Sequential mode makes ordering deterministic.

mod common;
use common::rfor;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temp dir and return (dir, path_as_string).
fn tmp() -> (TempDir, String) {
    let d = TempDir::new().unwrap();
    let p = d.path().to_str().unwrap().to_string();
    (d, p)
}

#[test]
fn halt_on_fail_stops_after_first_failure_sequential() {
    let (_dir, dir_path) = tmp();

    // 5 sequential jobs. Job 3 (item "c") fails. Jobs 4 and 5 should NOT run.
    // Each job creates a sentinel file named after its item.
    let template = format!(
        "sh -c 'touch {}/{{}}; if [ {{}} = c ]; then exit 1; fi'",
        dir_path
    );
    rfor()
        .args(["--halt-on-fail", &template, ":::", "a", "b", "c", "d", "e"])
        .assert()
        .failure();

    // Sentinel files a, b, c should exist (c ran but failed).
    assert!(PathBuf::from(format!("{}/a", dir_path)).exists(), "job a should have run");
    assert!(PathBuf::from(format!("{}/b", dir_path)).exists(), "job b should have run");
    assert!(PathBuf::from(format!("{}/c", dir_path)).exists(), "job c should have run (it's the failing one)");

    // d and e must NOT exist — they were never dispatched.
    assert!(!PathBuf::from(format!("{}/d", dir_path)).exists(), "job d should NOT have run after halt");
    assert!(!PathBuf::from(format!("{}/e", dir_path)).exists(), "job e should NOT have run after halt");
}

#[test]
fn halt_on_fail_exit_code_reflects_failure() {
    // When --halt-on-fail triggers, exit code should be non-zero.
    rfor()
        .args(["--halt-on-fail", "sh -c 'exit {}'", ":::", "0", "1", "0"])
        .assert()
        .failure();
}

#[test]
fn halt_on_fail_with_all_passing_exits_zero() {
    // --halt-on-fail with no failures should complete normally.
    let out = rfor()
        .args(["--halt-on-fail", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["a", "b", "c"]);
}

#[test]
fn halt_on_fail_first_job_fails_immediately() {
    let (_dir, dir_path) = tmp();

    // First job fails. Jobs 2 and 3 must not run.
    let template = format!(
        "sh -c 'touch {}/{{}}; if [ {{}} = x ]; then exit 1; fi'",
        dir_path
    );
    rfor()
        .args(["--halt-on-fail", &template, ":::", "x", "y", "z"])
        .assert()
        .failure();

    assert!(PathBuf::from(format!("{}/x", dir_path)).exists(), "failing job x should have run");
    assert!(!PathBuf::from(format!("{}/y", dir_path)).exists(), "job y should NOT have run");
    assert!(!PathBuf::from(format!("{}/z", dir_path)).exists(), "job z should NOT have run");
}

#[test]
fn halt_on_fail_parallel_stops_dispatching() {
    let (_dir, dir_path) = tmp();

    // With -j 2, the first batch (items 1,2) dispatches together.
    // Item 1 fails quickly. Items 3-6 should not be dispatched.
    // Items already running (2) may or may not complete — we only check
    // that at least some later items did NOT run.
    let template = format!(
        "sh -c 'touch {}/{{}}; if [ {{}} = 1 ]; then exit 1; else sleep 0.1; fi'",
        dir_path
    );
    rfor()
        .args(["-j", "2", "--halt-on-fail", &template, ":::", "1", "2", "3", "4", "5", "6"])
        .assert()
        .failure();

    // At minimum, items 5 and 6 should not have run (they were far back in queue).
    assert!(
        !PathBuf::from(format!("{}/5", dir_path)).exists()
            || !PathBuf::from(format!("{}/6", dir_path)).exists(),
        "later jobs (5/6) should not have been dispatched after halt"
    );
}
