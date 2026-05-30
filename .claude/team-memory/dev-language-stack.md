# Language & stack — rfor

**Decided by user, 2026-05-29.**

- **Language: Rust** (user picked Rust over Go/Python/Bash when given the choice).
- **Progress bar: `indicatif`** (user-specified).
- **Other crates: architect's call.** Current picks:
  - `clap` (derive macros) for CLI parsing
  - `crossbeam-channel` for job queue / output coordination
  - `std::io::IsTerminal` (Rust 1.70+) for TTY detection
  - `std::process::Command` for subprocess execution; line-buffered piped stdout/stderr via reader threads
  - No async runtime — threads + channels are sufficient and simpler

Do not switch languages or swap the progress-bar crate without re-asking the user.
