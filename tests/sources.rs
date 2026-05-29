//! Argument sources: `:::`, `::::`, stdin, and empty input.

mod common;
use common::pfor;
use std::io::Write;
use tempfile::NamedTempFile;

fn sorted_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.lines().map(|l| l.to_string()).collect();
    v.sort();
    v
}

#[test]
fn triple_colon_introduces_literal_args() {
    let out = pfor()
        .args(["echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}

#[test]
fn quadruple_colon_reads_items_from_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "one").unwrap();
    writeln!(f, "two").unwrap();
    writeln!(f, "three").unwrap();
    f.flush().unwrap();

    let out = pfor()
        .args(["echo {}", "::::"])
        .arg(f.path())
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["one", "three", "two"]);
}

#[test]
fn stdin_one_item_per_line() {
    let out = pfor()
        .arg("echo {}")
        .write_stdin("foo\nbar\nbaz\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["bar", "baz", "foo"]);
}

#[test]
fn stdin_without_trailing_newline_is_still_a_job() {
    let out = pfor()
        .arg("echo {}")
        .write_stdin("only")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "only");
}

#[test]
fn empty_stdin_produces_zero_jobs_and_exits_zero() {
    let out = pfor().arg("echo {}").write_stdin("").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected no job output, got: {:?}",
        stdout
    );
}

#[test]
fn empty_argfile_produces_zero_jobs_and_exits_zero() {
    let f = NamedTempFile::new().unwrap();
    pfor()
        .args(["echo {}", "::::"])
        .arg(f.path())
        .assert()
        .success();
}

#[test]
fn triple_colon_wins_over_stdin() {
    // Spec implies `:::` takes precedence over stdin. If implementer disagrees,
    // tester escalates to architect (see TEST_NOTES.md).
    let out = pfor()
        .args(["echo {}", ":::", "x", "y"])
        .write_stdin("from-stdin-1\nfrom-stdin-2\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines = sorted_lines(&stdout);
    assert_eq!(
        lines,
        vec!["x", "y"],
        "expected ::: args to win over stdin"
    );
}

#[test]
fn argfile_blank_lines_are_handled() {
    // Spec doesn't pin behavior. Acceptable choices: skip blank lines OR treat
    // each blank line as an empty-string item. This test accepts either, but
    // verifies the runner doesn't crash and total jobs in {1, 2}-ish range.
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "alpha").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "beta").unwrap();
    f.flush().unwrap();

    let out = pfor()
        .args(["echo [{}]", "::::"])
        .arg(f.path())
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let line_count = stdout.lines().count();
    assert!(
        (2..=3).contains(&line_count),
        "expected 2 or 3 output lines, got {}: {:?}",
        line_count,
        stdout
    );
}
