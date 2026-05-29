//! Tests for `--retries N` flag: retry failed jobs up to N additional times.
//!
//! Sprint 2 feature. Tests will fail until implementation lands.

mod common;
use common::pfor;
use tempfile::TempDir;

// ─── 1. Retry succeeds on second attempt ──────────────────────────────

#[test]
fn retries_passes_on_retry() {
    // Job fails the first time, succeeds the second.
    // Use a sentinel file: first run creates it and fails; second run
    // sees it exists and succeeds.
    let dir = TempDir::new().unwrap();
    let sentinel = dir.path().join("flag");
    let sentinel_str = sentinel.to_str().unwrap();

    // Template: if sentinel doesn't exist, create it and fail.
    // If it exists, succeed.
    let template = format!(
        "sh -c 'if [ ! -f {} ]; then touch {} && exit 1; else echo ok; fi'",
        sentinel_str, sentinel_str
    );

    let out = pfor()
        .args(["--retries", "2", &template, ":::", "x"])
        .assert()
        .success(); // Should succeed because retry #1 passes.
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("ok"), "retried job should eventually print 'ok'");
}

// ─── 2. Job that always fails exhausts retries ────────────────────────

#[test]
fn retries_exhausted_still_fails() {
    // `false` always fails. With --retries 2, it runs 3 times total
    // (1 original + 2 retries), then counts as 1 failure.
    pfor()
        .args(["--retries", "2", "false", ":::", "a"])
        .assert()
        .code(1); // 1 job failed.
}

// ─── 3. --retries 0 = no retries (default behavior) ──────────────────

#[test]
fn retries_zero_is_default_no_retry() {
    // --retries 0 is the same as not specifying --retries.
    // `false` fails once, exit code = 1.
    pfor()
        .args(["--retries", "0", "false", ":::", "a"])
        .assert()
        .code(1);
}

// ─── 4. Retries + halt-on-fail: halt only after retries exhausted ─────

#[test]
fn retries_with_halt_on_fail_waits_for_exhaustion() {
    // --retries 1 + --halt-on-fail: job "b" always fails. After 2 attempts
    // (1 + 1 retry) it's exhausted → halt triggers. Job "c" should NOT run.
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    let template = format!(
        "sh -c 'touch {}/{{}}; if [ {{}} = b ]; then exit 1; fi'",
        dir_str
    );

    pfor()
        .args(["--retries", "1", "--halt-on-fail", &template, ":::", "a", "b", "c"])
        .assert()
        .failure();

    // "a" should have run.
    assert!(
        dir.path().join("a").exists(),
        "job 'a' should have run"
    );
    // "b" should have run (it ran but failed, even with retries).
    assert!(
        dir.path().join("b").exists(),
        "job 'b' should have run (and failed)"
    );
    // "c" should NOT have run (halt triggered after 'b' exhausted retries).
    assert!(
        !dir.path().join("c").exists(),
        "job 'c' should NOT have run after halt"
    );
}

// ─── 5. Exit code reflects final results ──────────────────────────────

#[test]
fn retries_exit_code_reflects_final_success() {
    // 3 jobs: "a" always passes, "b" fails-then-passes (sentinel file trick),
    // "c" always passes. With --retries 2, all eventually pass → exit 0.
    let dir = TempDir::new().unwrap();
    let sentinel = dir.path().join("b_flag");
    let sentinel_str = sentinel.to_str().unwrap();

    let template = format!(
        "sh -c 'if [ {{}} = b ] && [ ! -f {} ]; then touch {} && exit 1; fi; echo {{}}'",
        sentinel_str, sentinel_str
    );

    pfor()
        .args(["--retries", "2", &template, ":::", "a", "b", "c"])
        .assert()
        .success(); // All eventually succeed.
}

// ─── 6. Stderr shows retry messages ───────────────────────────────────

#[test]
fn retries_stderr_shows_retry_info() {
    // When a job is retried, stderr should contain some indication
    // (e.g. "retrying", "retry", "attempt").
    let out = pfor()
        .args(["--retries", "1", "false", ":::", "x"])
        .assert()
        .failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    let stderr_lower = stderr.to_lowercase();
    assert!(
        stderr_lower.contains("retr"),
        "stderr should mention retrying, got: {:?}",
        stderr
    );
}

// ─── 7. Retries work under parallel ───────────────────────────────────

#[test]
fn retries_work_with_parallel() {
    // 4 jobs at -j 2, each failing once then succeeding (sentinel files).
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    let template = format!(
        "sh -c 'f={}/{{}}; if [ ! -f $f ]; then touch $f && exit 1; fi; echo ok-{{}}'",
        dir_str
    );

    let out = pfor()
        .args([
            "--retries", "2", "-j", "2",
            &template, ":::", "a", "b", "c", "d",
        ])
        .assert()
        .success(); // All pass after 1 retry each.
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    for item in ["a", "b", "c", "d"] {
        assert!(
            stdout.contains(&format!("ok-{}", item)),
            "parallel retried job '{}' should succeed",
            item
        );
    }
}

// ─── 8. Retries + group: only final attempt's output appears ──────────

#[test]
fn retries_with_group_shows_only_final_output() {
    // --retries + --group: output from failed attempts should be discarded;
    // only the final attempt's output (success or last failure) appears.
    let dir = TempDir::new().unwrap();
    let sentinel = dir.path().join("flag");
    let sentinel_str = sentinel.to_str().unwrap();

    // First attempt: prints "FAIL" and exits 1. Second attempt: prints "PASS".
    let template = format!(
        "sh -c 'if [ ! -f {} ]; then touch {} && echo FAIL && exit 1; else echo PASS; fi'",
        sentinel_str, sentinel_str
    );

    let out = pfor()
        .args(["--retries", "2", "--group", &template, ":::", "x"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // Only the final (successful) attempt's output should appear.
    assert!(
        stdout.contains("PASS"),
        "final attempt output 'PASS' should appear"
    );
    assert!(
        !stdout.contains("FAIL"),
        "failed attempt output 'FAIL' should be discarded with --group + --retries"
    );
}
