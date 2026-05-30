//! Shared helpers for rfor integration tests.
//!
//! All tests invoke the `rfor` binary as a black box via `assert_cmd`.
//! No test imports anything from the `rfor` crate's source.

#![allow(dead_code)]

use assert_cmd::Command;

/// Build a fresh `rfor` command.
pub fn rfor() -> Command {
    Command::cargo_bin("rfor").expect("rfor binary should be built by cargo")
}

/// ANSI escape introducer. Used to assert that off-TTY output is plain.
pub const ANSI_CSI: &str = "\x1b[";
