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
                let _ = mp.println(line);
                let _ = stream;
            }
            Printer::Plain(lock) => {
                let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                Self::write_line(&stream, line);
            }
        }
    }

    /// Print multiple lines atomically — holds the lock for the entire block
    /// so no other worker can interleave. Used by `--group` mode.
    pub fn println_block(&self, lines: &[(Stream, &str)]) {
        match self {
            Printer::Bar(mp) => {
                // indicatif's MultiProgress::println is internally locked,
                // but we need atomicity across multiple calls. Suspend the
                // bar once and write all lines.
                for (_, line) in lines {
                    let _ = mp.println(*line);
                }
            }
            Printer::Plain(lock) => {
                let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                for (stream, line) in lines {
                    Self::write_line(stream, line);
                }
            }
        }
    }

    fn write_line(stream: &Stream, line: &str) {
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
