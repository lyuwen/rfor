//! Tests for `do`/`done` keywords as bash-style separators.
//!
//! Sprint 4 feature. `do` works as an alternative to `--` for the command
//! separator in bash-style syntax. `done` is optional and ignored if present
//! (stripped from the command words).
//!
//! Syntax: `pfor VAR in ITEMS do COMMAND [done]`

mod common;
use common::pfor;

fn sorted_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.lines().map(|l| l.to_string()).collect();
    v.sort();
    v
}

// ─── 1. Basic do/done ─────────────────────────────────────────────────

#[test]
fn do_done_basic() {
    // `pfor i in a b c do echo {i} done` → outputs a, b, c
    let out = pfor()
        .args(["i", "in", "a", "b", "c", "do", "echo", "{i}", "done"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}

// ─── 2. Without trailing done ─────────────────────────────────────────

#[test]
fn do_without_trailing_done() {
    // `pfor i in a b c do echo {i}` → same result (done is optional)
    let out = pfor()
        .args(["i", "in", "a", "b", "c", "do", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}

// ─── 3. do with flags ─────────────────────────────────────────────────

#[test]
fn do_done_with_parallel_flag() {
    // `pfor -j 2 i in a b c d do echo {i} done`
    let out = pfor()
        .args(["-j", "2", "i", "in", "a", "b", "c", "d", "do", "echo", "{i}", "done"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c", "d"]);
}

// ─── 4. -- still works alongside do ──────────────────────────────────

#[test]
fn double_dash_still_works() {
    // `pfor i in a b c -- echo {i}` → unchanged behavior
    let out = pfor()
        .args(["i", "in", "a", "b", "c", "--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}

// ─── 5. Stdin mode with do ────────────────────────────────────────────

#[test]
fn do_done_stdin_mode() {
    // `echo items | pfor i do echo {i} done`
    let out = pfor()
        .args(["i", "do", "echo", "{i}", "done"])
        .write_stdin("foo\nbar\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["bar", "foo"]);
}

// ─── 6. Named var with do/done ────────────────────────────────────────

#[test]
fn do_done_named_var() {
    // `pfor file in a.txt b.txt do echo {file} done`
    let out = pfor()
        .args(["file", "in", "a.txt", "b.txt", "do", "echo", "{file}", "done"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a.txt", "b.txt"]);
}

// ─── 7. {#} with do/done ─────────────────────────────────────────────

#[test]
fn do_done_job_index() {
    // `pfor i in a b do echo {#} done`
    let out = pfor()
        .args(["i", "in", "a", "b", "do", "echo", "{#}", "done"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["1", "2"]);
}

// ─── 8. --halt-on-fail + do/done ──────────────────────────────────────

#[test]
fn do_done_with_halt_on_fail() {
    // Halt should work with do/done syntax.
    let out = pfor()
        .args([
            "--halt-on-fail",
            "i", "in", "ok", "fail", "after",
            "do",
            "if", "[", "{i}", "=", "fail", "];", "then", "exit", "1;", "fi;", "echo", "{i}",
            "done",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("ok"), "job 'ok' should have run");
    assert!(
        !stdout.contains("after"),
        "job 'after' should NOT have run after halt"
    );
}

// ─── 9. Multi-word command ────────────────────────────────────────────

#[test]
fn do_done_multi_word_command() {
    // `pfor i in a b do echo start {i} end done`
    let out = pfor()
        .args(["i", "in", "a", "b", "do", "echo", "start", "{i}", "end", "done"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(line.starts_with("start "), "expected 'start ...' got {:?}", line);
        assert!(line.ends_with(" end"), "expected '... end' got {:?}", line);
    }
}

// ─── 10. GNU parallel unchanged ───────────────────────────────────────

#[test]
fn gnu_parallel_unchanged_with_do_done_feature() {
    // `pfor 'echo {}' ::: a b c` still works — do/done doesn't break it.
    let out = pfor()
        .args(["echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}
