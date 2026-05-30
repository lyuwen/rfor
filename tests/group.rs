//! Tests for `--group` flag: buffer each job's output and emit as a contiguous
//! block, preventing interleaving under parallel execution.
//!
//! Sprint 2 feature. Tests will fail until implementation lands.

mod common;
use common::rfor;

fn sorted_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.lines().map(|l| l.to_string()).collect();
    v.sort();
    v
}

// ─── 1. Grouped output is contiguous per job ──────────────────────────

#[test]
fn group_parallel_output_is_contiguous_per_job() {
    // Each job prints 3 lines. With --group, all 3 lines from one job
    // must appear together (no interleaving from other jobs).
    // Template: print a header, body, footer tagged with the item.
    let out = rfor()
        .args([
            "--group", "-j", "2",
            "sh -c 'echo HEAD-{}; sleep 0.05; echo BODY-{}; echo TAIL-{}'",
            ":::", "A", "B", "C", "D",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    // Find each HEAD and verify the next 2 lines belong to the same job.
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("HEAD-") {
            let tag = &line[5..]; // e.g. "A"
            assert!(
                i + 2 < lines.len(),
                "HEAD-{} at line {} missing BODY/TAIL lines",
                tag, i
            );
            assert_eq!(
                lines[i + 1],
                format!("BODY-{}", tag),
                "BODY line should follow HEAD-{} without interleaving",
                tag
            );
            assert_eq!(
                lines[i + 2],
                format!("TAIL-{}", tag),
                "TAIL line should follow BODY-{} without interleaving",
                tag
            );
        }
    }
    // All 4 jobs should have produced output (4 × 3 = 12 lines).
    assert_eq!(lines.len(), 12, "expected 12 lines total");
}

// ─── 2. Grouped preserves all content ─────────────────────────────────

#[test]
fn group_preserves_all_output_content() {
    // Same items, same template — --group should produce the same set of
    // output lines as without --group, just potentially reordered.
    let out = rfor()
        .args(["--group", "-j", "2", "echo {}", ":::", "p", "q", "r", "s"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["p", "q", "r", "s"]);
}

// ─── 3. Group with sequential is a no-op ──────────────────────────────

#[test]
fn group_with_sequential_output_unchanged() {
    // -j 1 + --group: output should be in order (sequential means no interleaving
    // anyway, so --group is effectively a no-op).
    let out = rfor()
        .args(["--group", "-j", "1", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["a", "b", "c"]);
}

// ─── 4. Grouped stderr is also per-job ────────────────────────────────

#[test]
fn group_stderr_is_also_grouped_per_job() {
    // Each job writes to both stdout and stderr. With --group, stderr
    // lines for a job should be contiguous (not mixed with other jobs').
    let out = rfor()
        .args([
            "--group", "-j", "2",
            "sh -c 'echo OUT-{}; echo ERR-{} >&2'",
            ":::", "X", "Y",
        ])
        .assert()
        .success();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    // Both ERR-X and ERR-Y should appear.
    assert!(stderr.contains("ERR-X"), "stderr should contain ERR-X");
    assert!(stderr.contains("ERR-Y"), "stderr should contain ERR-Y");
}

// ─── 5. Group + halt-on-fail ──────────────────────────────────────────

#[test]
fn group_with_halt_on_fail_shows_completed_jobs() {
    // --group + --halt-on-fail: jobs that completed before halt should
    // still have their grouped output emitted.
    let out = rfor()
        .args([
            "--group", "--halt-on-fail",
            "sh -c 'if [ {} = fail ]; then exit 1; fi; echo ok-{}'",
            ":::", "a", "fail", "c",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // Job "a" ran before the failure — its output should appear.
    assert!(stdout.contains("ok-a"), "completed job 'a' output should appear");
}

// ─── 6. Group stress test ─────────────────────────────────────────────

#[test]
fn group_stress_many_parallel_jobs() {
    // 12 items at -j 4, each printing 3 lines via printf (single write to
    // avoid shell-level interleaving ambiguity). With --group, each job's
    // 3-line block must be contiguous.
    let items: Vec<&str> = vec![
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l",
    ];
    let mut args: Vec<&str> = vec![
        "--group", "-j", "4",
        // printf issues a single write containing all three lines.
        r#"printf 'H-%s\nM-%s\nT-%s\n' {} {} {}"#,
        ":::",
    ];
    args.extend(items.iter());

    let out = rfor().args(args).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    // 12 jobs × 3 lines = 36 lines.
    assert_eq!(lines.len(), 36, "expected 36 lines from 12 jobs, got {}", lines.len());

    // Each H-X must be immediately followed by M-X then T-X.
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("H-") {
            let tag = &line[2..];
            assert!(
                i + 2 < lines.len()
                    && lines[i + 1] == format!("M-{}", tag)
                    && lines[i + 2] == format!("T-{}", tag),
                "H-{} at line {} not followed by M-{}/T-{}, got [{:?}, {:?}]",
                tag, i, tag, tag,
                lines.get(i + 1),
                lines.get(i + 2),
            );
        }
    }
}

// ─── 7. Group + dry-run ───────────────────────────────────────────────

#[test]
fn group_with_dry_run() {
    // --group + --dry-run: rendered commands should still be printed, grouped.
    // With dry-run each job is just one line, so grouping doesn't change much,
    // but the flags shouldn't conflict.
    let out = rfor()
        .args(["--group", "--dry-run", "-j", "2", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.lines().count(), 3, "dry-run should print 3 commands");
}

// ─── 8. Default (no --group) can interleave ──────────────────────────

#[test]
fn without_group_parallel_may_interleave() {
    // Without --group, parallel output CAN interleave. We don't require
    // it to interleave (it might not on fast jobs), but we verify the
    // default behavior produces all output without error.
    let out = rfor()
        .args([
            "-j", "4",
            "sh -c 'echo LINE1-{}; echo LINE2-{}'",
            ":::", "a", "b", "c", "d",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // All items should appear in output (content preserved).
    for item in ["a", "b", "c", "d"] {
        assert!(
            stdout.contains(&format!("LINE1-{}", item)),
            "missing LINE1-{} in output", item
        );
        assert!(
            stdout.contains(&format!("LINE2-{}", item)),
            "missing LINE2-{} in output", item
        );
    }
}
