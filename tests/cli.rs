//! Surface-level CLI tests: help, version, missing-template errors.

mod common;
use common::pfor;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn help_flag_exits_zero_and_prints_usage() {
    let out = pfor().arg("--help").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(!stdout.trim().is_empty(), "--help stdout was empty");
}

#[test]
fn short_help_flag_exits_zero() {
    pfor().arg("-h").assert().success();
}

#[test]
fn version_flag_exits_zero_and_prints_something() {
    let out = pfor().arg("--version").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(!stdout.trim().is_empty(), "--version stdout was empty");
}

#[test]
fn short_version_flag_exits_zero() {
    pfor().arg("-V").assert().success();
}

#[test]
fn missing_command_template_is_an_error() {
    // No template and no input — must fail with a message on stderr.
    let out = pfor().write_stdin("").assert().failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        !stderr.trim().is_empty(),
        "expected an error message on stderr when template is missing"
    );
}

#[test]
fn help_mentions_jobs_flag() {
    // Light smoke check that --help describes the core flag. Tolerant phrasing.
    pfor()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("-j").or(contains("--jobs")));
}
