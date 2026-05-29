//! TTY detection for choosing bar vs. pass-through mode.

use std::io::{self, IsTerminal};

/// Returns true when stderr is a TTY. The progress bar renders to stderr,
/// so this is the right fd to gate on.
pub fn stderr_is_tty() -> bool {
    io::stderr().is_terminal()
}
