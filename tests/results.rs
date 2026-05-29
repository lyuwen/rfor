//! Tests for `--results DIR`: save each job's stdout/stderr to files.
//!
//! Sprint 3 feature. Tests will fail until implementation lands.

mod common;
use common::pfor;
use std::fs;
use tempfile::TempDir;

// ─── 1. Creates output files for each job ─────────────────────────────

#[test]
fn results_creates_output_files() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    pfor()
        .args(["--results", dir_str, "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();

    // There should be files in the results directory (one per job).
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.len() >= 3,
        "expected at least 3 result files, got {}",
        entries.len()
    );
}

// ─── 2. Files contain correct stdout content ──────────────────────────

#[test]
fn results_files_contain_correct_content() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    pfor()
        .args(["--results", dir_str, "echo hello-{}", ":::", "world"])
        .assert()
        .success();

    // Find the output file and check its content.
    let mut found_content = false;
    for entry in fs::read_dir(dir.path()).unwrap().filter_map(|e| e.ok()) {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        if content.contains("hello-world") {
            found_content = true;
            break;
        }
    }
    assert!(found_content, "expected a result file containing 'hello-world'");
}

// ─── 3. DIR is created if it doesn't exist ────────────────────────────

#[test]
fn results_creates_directory_if_missing() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("sub").join("dir");
    let nested_str = nested.to_str().unwrap();

    pfor()
        .args(["--results", nested_str, "echo {}", ":::", "x"])
        .assert()
        .success();

    assert!(nested.exists(), "results directory should have been created");
    let entries: Vec<_> = fs::read_dir(&nested)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!entries.is_empty(), "results directory should contain output files");
}

// ─── 4. Works with --group ────────────────────────────────────────────

#[test]
fn results_works_with_group() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    pfor()
        .args([
            "--results", dir_str, "--group", "-j", "2",
            "echo {}", ":::", "a", "b",
        ])
        .assert()
        .success();

    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.len() >= 2,
        "expected at least 2 result files with --group"
    );
}

// ─── 5. Works with --retries (only final attempt saved) ───────────────

#[test]
fn results_with_retries_saves_final_attempt() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();
    let sentinel = dir.path().join("sentinel");
    let sentinel_str = sentinel.to_str().unwrap();

    // Job fails first time (prints FAIL), succeeds second (prints PASS).
    let template = format!(
        "sh -c 'if [ ! -f {} ]; then touch {} && echo FAIL && exit 1; else echo PASS; fi'",
        sentinel_str, sentinel_str
    );

    let results_dir = TempDir::new().unwrap();
    let results_str = results_dir.path().to_str().unwrap();

    pfor()
        .args(["--results", results_str, "--retries", "2", &template, ":::", "x"])
        .assert()
        .success();

    // The saved result should contain PASS (final attempt), not FAIL.
    let mut found_pass = false;
    for entry in fs::read_dir(results_dir.path()).unwrap().filter_map(|e| e.ok()) {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        if content.contains("PASS") {
            found_pass = true;
        }
        assert!(
            !content.contains("FAIL"),
            "result file should not contain failed attempt output"
        );
    }
    assert!(found_pass, "result file should contain PASS from final attempt");
}

// ─── 6. Works with parallel ──────────────────────────────────────────

#[test]
fn results_works_with_parallel() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    pfor()
        .args([
            "--results", dir_str, "-j", "2",
            "echo {}", ":::", "p", "q", "r", "s",
        ])
        .assert()
        .success();

    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.len() >= 4,
        "expected at least 4 result files with -j 2"
    );
}

// ─── 7. Output still streams to terminal ──────────────────────────────

#[test]
fn results_still_streams_to_terminal() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    // --results saves to files AND output should still appear on stdout.
    let out = pfor()
        .args(["--results", dir_str, "echo {}", ":::", "visible"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("visible"),
        "output should still stream to stdout with --results, got: {:?}",
        stdout
    );
}

// ─── 8. Filename sanitization ─────────────────────────────────────────

#[test]
fn results_sanitizes_filenames() {
    let dir = TempDir::new().unwrap();
    let dir_str = dir.path().to_str().unwrap();

    // Items with slashes and special chars should be sanitized in filenames.
    pfor()
        .args([
            "--results", dir_str,
            "echo {}", ":::", "/path/to/file", "hello world", "a&b",
        ])
        .assert()
        .success();

    // All 3 jobs should have result files (names may be sanitized).
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.len() >= 3,
        "expected at least 3 result files for items with special chars, got {}",
        entries.len()
    );

    // No filename should contain a slash (they'd be subdirectories otherwise).
    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_str().unwrap();
        assert!(
            !name_str.contains('/'),
            "result filename should not contain slashes: {:?}",
            name_str
        );
    }
}
