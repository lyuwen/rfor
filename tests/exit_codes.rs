//! Exit code semantics: 0 on success, count of failures (capped at 125) otherwise.

mod common;
use common::pfor;

#[test]
fn all_jobs_succeed_exits_zero() {
    pfor()
        .args(["echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
}

#[test]
fn single_failure_exits_one() {
    // One job fails (false), two succeed (true). Exit code = 1.
    pfor()
        .args(["sh -c 'if [ {} = fail ]; then exit 1; fi'", ":::", "ok", "fail", "ok"])
        .assert()
        .code(1);
}

#[test]
fn multiple_failures_exit_code_equals_count() {
    // 3 out of 5 fail. Exit code = 3.
    pfor()
        .args([
            "sh -c 'case {} in f*) exit 1;; esac'",
            ":::", "ok1", "f1", "f2", "ok2", "f3",
        ])
        .assert()
        .code(3);
}

#[test]
fn all_jobs_fail_exit_code_equals_total() {
    // 4 jobs, all fail. Exit code = 4.
    pfor()
        .args(["false", ":::", "a", "b", "c", "d"])
        .assert()
        .code(4);
}

#[test]
fn failure_count_caps_at_125() {
    // Spec says exit code is capped at 125. Create 130 failing jobs.
    let items: Vec<String> = (1..=130).map(|i| i.to_string()).collect();
    let mut args: Vec<&str> = vec!["-j", "8", "false", ":::"];
    args.extend(items.iter().map(|s| s.as_str()));
    pfor().args(args).assert().code(125);
}

#[test]
fn mixed_exit_codes_counted_as_one_failure_each() {
    // Jobs exiting with different non-zero codes each count as 1 failure.
    // 3 jobs: exit 0, exit 2, exit 5. Failure count = 2.
    pfor()
        .args([
            "sh -c 'exit {}'",
            ":::", "0", "2", "5",
        ])
        .assert()
        .code(2);
}

#[test]
fn zero_jobs_exits_zero() {
    // No input → no jobs → success.
    pfor().arg("echo {}").write_stdin("").assert().success();
}
