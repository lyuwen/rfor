//! Tests for `--dry-run` flag: prints rendered commands without executing.
//!
//! Sprint 1 feature. Tests will fail until implementation lands.

mod common;
use common::pfor;

#[test]
fn dry_run_prints_commands_not_output() {
    // `--dry-run` should print the rendered shell commands, NOT execute them.
    // `pfor --dry-run 'echo {}' ::: a b c` should output rendered commands
    // containing "echo" and the item, NOT just "a", "b", "c".
    let out = pfor()
        .args(["--dry-run", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 rendered commands, got: {:?}", lines);
    // Each line should contain "echo" (it's the rendered command, not the output).
    for line in &lines {
        assert!(
            line.contains("echo"),
            "dry-run output should show the rendered command containing 'echo', got: {:?}",
            line
        );
    }
    // The items should appear in the commands (shell-quoted or not).
    let joined = stdout.to_string();
    assert!(joined.contains("a"), "rendered command should contain item 'a'");
    assert!(joined.contains("b"), "rendered command should contain item 'b'");
    assert!(joined.contains("c"), "rendered command should contain item 'c'");
}

#[test]
fn dry_run_exit_code_is_always_zero() {
    // Even with commands that would fail, --dry-run never executes, so exit 0.
    pfor()
        .args(["--dry-run", "false", ":::", "a", "b", "c"])
        .assert()
        .success();
}

#[test]
fn dry_run_works_with_parallel() {
    // --dry-run + -j 2 should still just print commands (no execution).
    let out = pfor()
        .args(["--dry-run", "-j", "2", "echo {}", ":::", "a", "b", "c"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.sort();
    assert_eq!(lines.len(), 3, "expected 3 rendered commands");
    // All items should appear across the output.
    let joined = stdout.to_string();
    for item in ["a", "b", "c"] {
        assert!(joined.contains(item), "missing item '{}' in dry-run output", item);
    }
}

#[test]
fn dry_run_works_with_bash_style() {
    // `pfor --dry-run i in a b c -- echo {i}` should print rendered commands.
    let out = pfor()
        .args(["--dry-run", "i", "in", "a", "b", "c", "--", "echo", "{i}"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 rendered commands");
    for line in &lines {
        assert!(
            line.contains("echo"),
            "bash-style dry-run should show 'echo' in rendered command, got: {:?}",
            line
        );
    }
}

#[test]
fn dry_run_shows_index_in_rendered_command() {
    // `{#}` should be expanded in the dry-run output.
    let out = pfor()
        .args(["--dry-run", "echo {#}: {}", ":::", "x", "y"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    // The rendered commands should contain the expanded indices "1" and "2".
    let joined = stdout.to_string();
    assert!(joined.contains("1"), "dry-run should expand {{#}} to index 1");
    assert!(joined.contains("2"), "dry-run should expand {{#}} to index 2");
}

#[test]
fn dry_run_with_halt_on_fail_no_conflict() {
    // --dry-run + --halt-on-fail should not conflict — nothing executes,
    // nothing can fail, so halt-on-fail is a no-op. Exit 0.
    let out = pfor()
        .args([
            "--dry-run", "--halt-on-fail",
            "might-fail {}", ":::", "a", "b", "c",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.lines().count(), 3, "should still print all 3 commands");
}
