//! Tests for filename tokens: `{.}`, `{/}`, `{//}`, `{/.}`.
//!
//! Sprint 1 feature (GNU parallel compatibility tokens).
//! Tests will fail until implementation lands.

mod common;
use common::pfor;

fn sorted_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.lines().map(|l| l.to_string()).collect();
    v.sort();
    v
}

// ─── {.} strips extension ─────────────────────────────────────────────

#[test]
fn dot_token_strips_extension() {
    // `{.}` = item without the last extension.
    // `photo.jpg` → `photo`
    let out = pfor()
        .args(["echo {.}", ":::", "photo.jpg"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "photo");
}

#[test]
fn dot_token_strips_only_last_extension() {
    // `archive.tar.gz` → `archive.tar` (only last `.gz` stripped).
    let out = pfor()
        .args(["echo {.}", ":::", "archive.tar.gz"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "archive.tar");
}

// ─── {/} basename ─────────────────────────────────────────────────────

#[test]
fn slash_token_gives_basename() {
    // `{/}` = basename of the item.
    // `/path/to/file.txt` → `file.txt`
    let out = pfor()
        .args(["echo {/}", ":::", "/path/to/file.txt"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "file.txt");
}

// ─── {//} dirname ─────────────────────────────────────────────────────

#[test]
fn double_slash_token_gives_dirname() {
    // `{//}` = directory part of the item.
    // `/path/to/file.txt` → `/path/to`
    let out = pfor()
        .args(["echo {//}", ":::", "/path/to/file.txt"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "/path/to");
}

// ─── {/.} basename without extension ──────────────────────────────────

#[test]
fn slash_dot_token_gives_basename_no_ext() {
    // `{/.}` = basename without extension.
    // `/path/to/file.txt` → `file`
    let out = pfor()
        .args(["echo {/.}", ":::", "/path/to/file.txt"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "file");
}

// ─── Edge: no extension ───────────────────────────────────────────────

#[test]
fn dot_token_no_extension_returns_item() {
    // `{.}` on an item without an extension → returns the item unchanged.
    let out = pfor()
        .args(["echo {.}", ":::", "noext"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "noext");
}

// ─── Edge: no directory ───────────────────────────────────────────────

#[test]
fn double_slash_token_no_directory_returns_dot() {
    // `{//}` on a bare filename → `.` (current directory, matching GNU parallel).
    let out = pfor()
        .args(["echo {//}", ":::", "file.txt"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // GNU parallel returns `.` for bare filenames.
    assert_eq!(stdout.trim(), ".");
}

// ─── All tokens together ──────────────────────────────────────────────

#[test]
fn all_tokens_together() {
    // `pfor 'echo {} {.} {/} {//} {/.}' ::: /tmp/test.log`
    // Expected: `/tmp/test.log /tmp/test test.log /tmp test`
    let out = pfor()
        .args(["echo {} {.} {/} {//} {/.}", ":::", "/tmp/test.log"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    assert_eq!(
        parts,
        vec!["/tmp/test.log", "/tmp/test", "test.log", "/tmp", "test"],
        "all filename tokens should expand correctly"
    );
}

// ─── Filename tokens with bash-style ──────────────────────────────────

#[test]
fn filename_tokens_with_bash_style() {
    // `pfor f in a.txt b.jpg -- echo {.}`
    // Should strip extensions: `a` and `b`
    let out = pfor()
        .args(["f", "in", "a.txt", "b.jpg", "--", "echo", "{.}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(sorted_lines(&stdout), vec!["a", "b"]);
}

// ─── Existing tokens still work alongside filename tokens ─────────────

#[test]
fn existing_tokens_work_alongside_filename_tokens() {
    // `{}` and `{#}` must still work when filename tokens are also present.
    let out = pfor()
        .args(["echo {#} {} {.}", ":::", "doc.pdf"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    assert_eq!(
        parts,
        vec!["1", "doc.pdf", "doc"],
        "{{#}}, {{}}, and {{.}} should all work in the same template"
    );
}
