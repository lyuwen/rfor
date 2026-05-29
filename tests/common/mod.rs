//! Shared helpers for pfor integration tests.
//!
//! All tests invoke the `pfor` binary as a black box via `assert_cmd`.
//! No test imports anything from the `pfor` crate's source.

#![allow(dead_code)]

use assert_cmd::Command;

/// Build a fresh `pfor` command.
pub fn pfor() -> Command {
    Command::cargo_bin("pfor").expect("pfor binary should be built by cargo")
}

/// ANSI escape introducer. Used to assert that off-TTY output is plain.
pub const ANSI_CSI: &str = "\x1b[";
