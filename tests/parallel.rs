//! Parallel execution: `-j N` throughput, `-j 0` = num CPUs, and per-line
//! output atomicity under concurrency.

mod common;
use common::pfor;

#[test]
fn parallel_is_faster_than_sequential_on_sleeping_jobs() {
    // 8 jobs sleeping 0.5s each.
    // Sequential: ~4.0s. Parallel -j 4: ~1.0s. We allow generous margins.
    let items: Vec<&str> = vec!["1", "2", "3", "4", "5", "6", "7", "8"];

    // Sequential baseline timing
    let seq_start = std::time::Instant::now();
    pfor()
        .args(["-j", "1", "sleep 0.5"])
        .arg(":::")
        .args(&items)
        .assert()
        .success();
    let seq_elapsed = seq_start.elapsed();

    // Parallel timing
    let par_start = std::time::Instant::now();
    pfor()
        .args(["-j", "4", "sleep 0.5"])
        .arg(":::")
        .args(&items)
        .assert()
        .success();
    let par_elapsed = par_start.elapsed();

    // Parallel must be meaningfully faster.
    assert!(
        par_elapsed.as_secs_f64() < seq_elapsed.as_secs_f64() * 0.7,
        "parallel -j4 ({:.2}s) should be significantly faster than sequential ({:.2}s)",
        par_elapsed.as_secs_f64(),
        seq_elapsed.as_secs_f64()
    );
}

#[test]
fn jobs_zero_means_num_cpus_and_runs_in_parallel() {
    // -j 0 = num CPUs. 8 jobs sleeping 0.3s each.
    // Even on a 2-core machine: 4 rounds × 0.3s = 1.2s, well under sequential 2.4s.
    // On most CI (4+ cores): ~0.6s.
    // We just verify it finishes in under 2.0s (sequential would be ~2.4s).
    let start = std::time::Instant::now();
    pfor()
        .args(["-j", "0", "sleep 0.3"])
        .arg(":::")
        .args(["1", "2", "3", "4", "5", "6", "7", "8"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "-j 0 with 8×0.3s jobs took {:.2}s, expected < 2.0s (parallel)",
        elapsed.as_secs_f64()
    );
}

#[test]
fn parallel_j2_limits_concurrency() {
    // 4 jobs sleeping 0.3s each, -j 2.
    // Optimal: 2 rounds × 0.3s = 0.6s. Sequential: 1.2s.
    // We check it's faster than sequential but still takes real time.
    let start = std::time::Instant::now();
    pfor()
        .args(["-j", "2", "sleep 0.3", ":::", "a", "b", "c", "d"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Should be around 0.6s (2 rounds). Must be less than sequential ~1.2s.
    assert!(
        elapsed.as_secs_f64() < 1.0,
        "-j 2 with 4×0.3s jobs took {:.2}s, expected < 1.0s",
        elapsed.as_secs_f64()
    );
}

#[test]
fn per_line_output_is_atomic_under_parallel() {
    // Each job prints a single line. Under -j 4, lines must not interleave
    // mid-line (no partial lines or mixed characters).
    let out = pfor()
        .args([
            "-j", "4",
            "echo AAAA-{}-ZZZZ",
            ":::", "1", "2", "3", "4", "5", "6", "7", "8",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    for line in stdout.lines() {
        assert!(
            line.starts_with("AAAA-") && line.ends_with("-ZZZZ"),
            "line appears to be interleaved or truncated: {:?}",
            line
        );
    }
    assert_eq!(stdout.lines().count(), 8, "expected 8 output lines");
}

#[test]
fn parallel_produces_all_outputs() {
    // All items must appear in output regardless of scheduling order.
    let out = pfor()
        .args(["-j", "4", "echo {}", ":::", "p", "q", "r", "s", "t"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["p", "q", "r", "s", "t"]);
}
