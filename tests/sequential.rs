//! Sequential execution (`-j 1`, the default): correct ordering and
//! one-at-a-time behavior.

mod common;
use common::rfor;

#[test]
fn sequential_output_is_in_order() {
    // With -j 1 (default), jobs must run in the order items are given,
    // so output lines match input order.
    let out = rfor()
        .args(["echo {}", ":::", "first", "second", "third"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["first", "second", "third"]);
}

#[test]
fn sequential_default_is_j1() {
    // Omitting -j should behave identically to -j 1 (ordered).
    let out = rfor()
        .args(["echo {}", ":::", "a", "b", "c", "d"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["a", "b", "c", "d"]);
}

#[test]
fn sequential_explicit_j1_same_as_default() {
    let out = rfor()
        .args(["-j", "1", "echo {}", ":::", "x", "y", "z"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["x", "y", "z"]);
}

#[test]
fn sequential_preserves_index_order() {
    // {#} indices must be strictly sequential 1,2,3,… with -j 1.
    let out = rfor()
        .args(["echo {#}", ":::", "a", "b", "c", "d", "e"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1", "2", "3", "4", "5"]);
}

#[test]
fn sequential_is_truly_serial() {
    // Run 4 jobs that each sleep 0.2s. With -j 1, wall time ≥ 0.8s.
    // With parallel, it would be ~0.2s. We check ≥ 0.6s to give margin.
    let start = std::time::Instant::now();
    rfor()
        .args(["-j", "1", "sleep 0.2", ":::", "1", "2", "3", "4"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs_f64() >= 0.6,
        "sequential 4×0.2s should take ≥0.6s, took {:.2}s",
        elapsed.as_secs_f64()
    );
}
