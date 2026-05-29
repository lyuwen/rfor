//! Tests for bash for-loop style syntax: `pfor VAR in ITEMS -- COMMAND`.
//!
//! Spec: `.claude/team-memory/dev-pfor-bash-syntax-scope.md`
//! 12 scenarios covering basic usage, named variable resolution, backward
//! compatibility tokens, stdin/argfile modes, flag positioning, error cases,
//! and coexistence with GNU-parallel syntax.

mod common;
use common::pfor;
use std::io::Write;
use tempfile::NamedTempFile;

fn sorted_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.lines().map(|l| l.to_string()).collect();
    v.sort();
    v
}

// ─── 1. Basic bash-style syntax ────────────────────────────────────────

#[test]
fn bash_style_basic_inline_items() {
    // `pfor i in a b c -- echo {i}` → prints a, b, c
    let out = pfor()
        .args(["i", "in", "a", "b", "c", "--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}

// ─── 2. Named variable substitution ───────────────────────────────────

#[test]
fn bash_style_named_var_substitutes_correctly() {
    // `{myvar}` resolves when the declared variable is `myvar`.
    let out = pfor()
        .args(["myvar", "in", "hello", "world", "--", "echo", "{myvar}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["hello", "world"]);
}

// ─── 3. Wrong variable name passes through literally ──────────────────

#[test]
fn bash_style_wrong_var_name_passes_through() {
    // Declared var is `i`, but template uses `{other}` → literal `{other}`.
    let out = pfor()
        .args(["i", "in", "x", "--", "echo", "{other}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "{other}");
}

// ─── 4. `{}` unnamed placeholder backward compat ──────────────────────

#[test]
fn bash_style_unnamed_placeholder_still_works() {
    // `{}` always substitutes the current item, even in bash-style.
    let out = pfor()
        .args(["i", "in", "alpha", "beta", "--", "echo", "{}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["alpha", "beta"]);
}

// ─── 5. `{#}` job index backward compat ──────────────────────────────

#[test]
fn bash_style_job_index_placeholder_still_works() {
    // `{#}` gives 1-based job index, even in bash-style mode.
    let out = pfor()
        .args(["i", "in", "a", "b", "c", "--", "echo", "{#}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["1", "2", "3"]);
}

// ─── 6. Stdin mode ────────────────────────────────────────────────────

#[test]
fn bash_style_stdin_items() {
    // `cat items | pfor i -- echo {i}` — no `in`, items from stdin.
    let out = pfor()
        .args(["i", "--", "echo", "{i}"])
        .write_stdin("foo\nbar\nbaz\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["bar", "baz", "foo"]);
}

// ─── 7. Argfile mode via `::::` ───────────────────────────────────────

#[test]
fn bash_style_argfile_with_quadruple_colon() {
    // `pfor i in :::: file.txt -- echo {i}`
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "line1").unwrap();
    writeln!(f, "line2").unwrap();
    writeln!(f, "line3").unwrap();
    f.flush().unwrap();

    let out = pfor()
        .args(["i", "in", "::::"])
        .arg(f.path())
        .args(["--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["line1", "line2", "line3"]);
}

// ─── 8. Flags before the variable name ────────────────────────────────

#[test]
fn bash_style_flags_before_variable() {
    // `pfor -j 2 i in a b c d -- echo {i}` — flags come first.
    let out = pfor()
        .args(["-j", "2", "i", "in", "a", "b", "c", "d", "--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c", "d"]);
}

// ─── 9. `--halt-on-fail` with bash-style ──────────────────────────────

#[test]
fn bash_style_halt_on_fail() {
    // Sequential default: "ok" runs, "fail" exits 1 → halt, "after" should NOT run.
    // Template words are joined into: `if [ {i} = fail ]; then exit 1; fi; echo {i}`
    // pfor wraps in `sh -c`, so no need for an explicit `sh -c` in the template.
    let out = pfor()
        .args([
            "--halt-on-fail",
            "i", "in", "ok", "fail", "after",
            "--",
            "if", "[", "{i}", "=", "fail", "];", "then", "exit", "1;", "fi;", "echo", "{i}",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // "ok" should appear, "after" should NOT (halted after "fail").
    assert!(stdout.contains("ok"), "job 'ok' should have run");
    assert!(
        !stdout.contains("after"),
        "job 'after' should NOT have run after halt, got: {:?}",
        stdout
    );
}

// ─── 10. Error: missing `--` separator ────────────────────────────────

#[test]
fn bash_style_missing_separator_is_error() {
    // `pfor i in a b c echo {i}` — no `--` to delimit the command.
    // This should either error or fall through to GNU-parallel mode (which
    // would also error since "i" isn't a valid template). Either way: failure.
    pfor()
        .args(["i", "in", "a", "b", "c", "echo", "{i}"])
        .assert()
        .failure();
}

// ─── 11. Error: no command after `--` ─────────────────────────────────

#[test]
fn bash_style_no_command_after_separator_is_error() {
    // `pfor i in a b c --` — nothing after the separator.
    pfor()
        .args(["i", "in", "a", "b", "c", "--"])
        .assert()
        .failure();
}

// ─── 12. GNU-parallel syntax still works alongside bash-style ─────────

#[test]
fn gnu_parallel_syntax_still_works() {
    // The existing `pfor 'echo {}' ::: a b c` must continue to work.
    let out = pfor()
        .args(["echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b", "c"]);
}
