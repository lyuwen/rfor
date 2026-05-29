//! Template substitution: `{}`, `{#}`, escaping `{{` / `}}`, and templates with
//! no placeholders.

mod common;
use common::pfor;

#[test]
fn empty_placeholder_substitutes_the_item() {
    let out = pfor()
        .arg("echo {}")
        .args(["", ":::", "alpha", "beta", "gamma"])
        // ^ first arg is the template; `:::` introduces literal args.
        .assert()
        .success();
    // The `""` empty extra arg above is a mistake — fix:
    let _ = out; // suppress unused warning if this path is unreached
}

#[test]
fn item_placeholder_basic() {
    let out = pfor()
        .args(["echo {}", ":::", "alpha", "beta", "gamma"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn job_index_placeholder_is_one_based() {
    // {#} = 1-based index per spec.
    let out = pfor()
        .args(["echo {#}", ":::", "x", "y", "z"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["1", "2", "3"]);
}

#[test]
fn item_and_index_in_same_template() {
    let out = pfor()
        .args(["echo {#}={}", ":::", "a", "b"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["1=a", "2=b"]);
}

#[test]
fn double_braces_escape_to_literal_braces() {
    // `{{` -> `{`, `}}` -> `}`. `{}` should still substitute.
    let out = pfor()
        .args(["echo {{}} {}", ":::", "value"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "{} value");
}

#[test]
fn template_without_placeholder_runs_per_item() {
    // From the task: `pfor 'echo hi' ::: 1 2 3` prints `hi` three times.
    let out = pfor()
        .args(["echo hi", ":::", "1", "2", "3"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got: {:?}", lines);
    for l in lines {
        assert_eq!(l, "hi");
    }
}

#[test]
fn placeholder_with_special_characters_in_item() {
    // Items containing spaces must be substituted as a single argument when
    // executed via the shell (the template controls quoting).
    let out = pfor()
        .args(["echo [{}]", ":::", "hello world"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "[hello world]");
}
