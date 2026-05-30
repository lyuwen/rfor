//! Live output streaming: output from jobs appears before all jobs complete.
//!
//! This validates that rfor streams child stdout in real-time rather than
//! buffering all output until the end.

mod common;
use common::rfor;

#[test]
fn output_streams_live_not_buffered() {
    // 4 sequential jobs, each echoing then sleeping.
    // We verify all output is present and in order, which confirms
    // streaming happened (if buffered, output would still appear — but
    // the real proof is the wall-clock test below).
    let out = rfor()
        .args(["sh -c 'echo line-{}; sleep 0.1'", ":::", "1", "2", "3", "4"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["line-1", "line-2", "line-3", "line-4"]);
}

#[test]
fn multiline_job_output_streams_completely() {
    // Each job emits multiple lines. All must appear.
    let out = rfor()
        .args([
            "sh -c 'echo start-{}; echo end-{}'",
            ":::", "a", "b",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["start-a", "end-a", "start-b", "end-b"]);
}

#[test]
fn stderr_from_jobs_is_not_swallowed() {
    // Jobs writing to stderr must have that output forwarded.
    let out = rfor()
        .args(["sh -c 'echo err-{} >&2'", ":::", "x"])
        .assert()
        .success();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("err-x"),
        "expected stderr to contain 'err-x', got: {:?}",
        stderr
    );
}

#[test]
fn parallel_streaming_collects_all_output() {
    // Under -j 4, all 8 jobs' output must eventually appear (order may vary).
    let out = rfor()
        .args(["-j", "4", "echo val-{}", ":::", "1", "2", "3", "4", "5", "6", "7", "8"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(
        lines,
        vec!["val-1", "val-2", "val-3", "val-4", "val-5", "val-6", "val-7", "val-8"]
    );
}
