//! Serialized live output printer.
//!
//! Each child process has two reader threads (stdout, stderr) that
//! forward complete lines to the printer. The printer is the single
//! point that touches the terminal, so lines never interleave and the
//! sticky progress bar (rendered by indicatif) stays uncorrupted.
//!
//! Two modes:
//! - `Bar`: use `MultiProgress::println` so each line redraws above the bar.
//! - `Plain`: write directly to stdout/stderr (no bar in use).

use indicatif::MultiProgress;
use std::io::{self, Write};
use std::sync::Mutex;

pub enum Stream {
    Stdout,
    Stderr,
}

/// Where lines should be sent.
pub enum Printer {
    /// Bar mode: route through MultiProgress so the bar redraws above the line.
    Bar(MultiProgress),
    /// Plain mode (no TTY): write directly. A mutex serializes writers so
    /// lines from concurrent jobs don't interleave at the byte level.
    Plain(Mutex<()>),
}

impl Printer {
    /// Print one complete line (no trailing newline included) on `stream`.
    /// The printer adds the newline.
    pub fn println(&self, stream: Stream, line: &str) {
        match self {
            Printer::Bar(mp) => {
                // indicatif renders the bar on stderr; printing through
                // MultiProgress::println suspends the bar, writes the line
                // to stderr, and redraws. We intentionally collapse stdout
                // and stderr to the same channel here so both stream above
                // the bar; if a user wants strict stdout/stderr separation
                // they should redirect off-TTY (then we fall to Plain).
                let _ = mp.println(line);
                let _ = stream; // both routed via mp.println for ordering
            }
            Printer::Plain(lock) => {
                let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                match stream {
                    Stream::Stdout => {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                    Stream::Stderr => {
                        let mut err = io::stderr().lock();
                        let _ = writeln!(err, "{}", line);
                        let _ = err.flush();
                    }
                }
            }
        }
    }
}
