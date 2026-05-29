//! Tests for brace expansion: `{N..M}`, `{N..M..S}`, `{a..z}`, zero-padded.
//!
//! Sprint 3 feature. Brace expansion happens at the argument source level —
//! items like `{1..5}` expand into `1 2 3 4 5` before template substitution.

mod common;
use common::pfor;

// ─── 1. Numeric range ─────────────────────────────────────────────────

#[test]
fn brace_expansion_numeric_range() {
    // `{1..5}` → items 1, 2, 3, 4, 5
    let out = pfor()
        .args(["echo {}", ":::", "{1..5}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1", "2", "3", "4", "5"]);
}

// ─── 2. Reverse numeric range ─────────────────────────────────────────

#[test]
fn brace_expansion_reverse_range() {
    // `{5..1}` → items 5, 4, 3, 2, 1
    let out = pfor()
        .args(["echo {}", ":::", "{5..1}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["5", "4", "3", "2", "1"]);
}

// ─── 3. Step ──────────────────────────────────────────────────────────

#[test]
fn brace_expansion_with_step() {
    // `{1..10..3}` → items 1, 4, 7, 10
    let out = pfor()
        .args(["echo {}", ":::", "{1..10..3}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1", "4", "7", "10"]);
}

// ─── 4. Zero-padded ──────────────────────────────────────────────────

#[test]
fn brace_expansion_zero_padded() {
    // `{01..05}` → items 01, 02, 03, 04, 05
    let out = pfor()
        .args(["echo {}", ":::", "{01..05}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["01", "02", "03", "04", "05"]);
}

// ─── 5. Alphabetic range ─────────────────────────────────────────────

#[test]
fn brace_expansion_alphabetic() {
    // `{a..e}` → items a, b, c, d, e
    let out = pfor()
        .args(["echo {}", ":::", "{a..e}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["a", "b", "c", "d", "e"]);
}

// ─── 6. Mixed with regular items ─────────────────────────────────────

#[test]
fn brace_expansion_mixed_with_regular_items() {
    // `{1..3} hello` → items 1, 2, 3, hello
    let out = pfor()
        .args(["echo {}", ":::", "{1..3}", "hello"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1", "2", "3", "hello"]);
}

// ─── 7. Works with bash-style ────────────────────────────────────────

#[test]
fn brace_expansion_with_bash_style() {
    // `pfor i in {1..3} -- echo {i}` → outputs 1, 2, 3
    let out = pfor()
        .args(["i", "in", "{1..3}", "--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1", "2", "3"]);
}

// ─── 8. Non-range pattern passes through ─────────────────────────────

#[test]
fn non_range_brace_pattern_passes_through() {
    // `{notarange}` should not be expanded — treated as a literal item.
    let out = pfor()
        .args(["echo {}", ":::", "{notarange}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "{notarange}");
}
