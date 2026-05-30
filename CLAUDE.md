# CLAUDE.md — rfor Development Guidelines

## Project Overview

**rfor** (Rust for) is a parallel for-loop replacement CLI tool with live output and a sticky progress bar. It supports both GNU parallel–style syntax and bash for-loop–style syntax with real named variables.

```bash
# GNU parallel style
rfor 'echo {}' ::: a b c

# Bash for-loop style
rfor i in a b c do echo {i} done

# Full power
rfor -j 8 --multi-bar --group --retries 3 --results output/ \
  f in {01..100} do convert {f} {.}.jpg done
```

## Architecture

The codebase follows a clean pipeline: `cli → source → expand → template → runner → output`.

```
src/
├── main.rs       — Entry point, wires everything together
├── cli.rs        — clap derive CLI, arg parsing, syntax detection (GNU vs bash-style)
├── expand.rs     — Brace expansion ({1..10}, {a..z}, {01..05})
├── template.rs   — Token substitution ({}, {#}, {.}, {/}, {//}, {/.}, {var})
├── source.rs     — Item source resolution (:::, ::::, stdin)
├── runner.rs     — Job execution (sequential/parallel, retries, grouping, results)
├── output.rs     — Printer abstraction (Bar/Plain modes, atomic output blocks)
└── tty.rs        — TTY detection for stderr
```

### Key Design Decisions

- **No async runtime** — `std::thread` + `crossbeam-channel` for simplicity and predictability
- **Shell quoting** — all token substitutions use POSIX `shell_quote()` to prevent injection
- **Dual syntax** — auto-detected by examining positional args: `{}` or `{#}` in first arg → GNU-parallel; `rest[1] == "in"` → bash-style; `rest[1] == "--"` or `"do"` → bash stdin
- **Separator precedence** — `--` takes priority over `do` so literal "do" items work with `--`
- **Named variables** — `{varname}` only substitutes when the name matches the declared variable; unmatched `{name}` passes through literally
- **Progress bar** — `indicatif` on stderr (TTY-gated). `ProgressBar::println()` keeps output above the sticky bar
- **Atomic grouped output** — `Mutex<()>` on both `Printer::Bar` and `Printer::Plain` variants ensures `println_block()` is atomic even in Bar mode (this was a MUST-FIX from code review — do not regress)

## Building and Testing

```bash
# Build
cargo build

# Release build (LTO + stripped)
cargo build --release

# Run all tests (179 tests)
cargo test

# Clippy (must be zero warnings)
cargo clippy

# Install locally
cargo install --path .
```

### Test Organization

Tests are black-box integration tests using `assert_cmd` — no internal imports from the rfor crate. Each feature has its own test file:

```
tests/
├── bash_style.rs       — Bash for-loop syntax + named variables
├── brace_expansion.rs  — {1..10}, {a..z}, {01..05} ranges
├── cli.rs              — --help, --version, missing args
├── do_done.rs          — do/done keyword separators
├── dry_run.rs          — --dry-run flag behavior
├── exit_codes.rs       — Exit code semantics (0, 1-125, cap)
├── filename_tokens.rs  — {.}, {/}, {//}, {/.} tokens
├── group.rs            — --group buffered output atomicity
├── halt_on_fail.rs     — --halt-on-fail with sentinel files
├── multi_bar.rs        — --multi-bar per-worker display
├── parallel.rs         — -j N parallel execution
├── results.rs          — --results DIR file output
├── retries.rs          — --retries N retry logic
├── sequential.rs       — Default sequential execution
├── sources.rs          — :::, ::::, stdin item sources
├── streaming.rs        — Live output streaming
├── template.rs         — Token substitution
└── tty.rs              — TTY detection, no ANSI when piped
```

## Development Principles

### Code Quality

- **Zero clippy warnings** — enforced throughout. No `#[allow]` unless absolutely necessary (one exception: `too_many_arguments` on `worker_loop` was resolved with `WorkerCtx` struct).
- **No `unwrap()` in production code** — all error paths produce user-facing messages and exit code 2.
- **Doc comments on all public items** — modules start with `//!` doc comments.

### Safety and Correctness

- **Shell injection prevention** — all token substitutions go through `shell_quote()` using POSIX single-quote wrapping with `'\''` escape.
- **Output atomicity** — `Printer::println_block()` holds a mutex for the entire block in BOTH Bar and Plain modes. This was the only MUST-FIX blocker found during development. Do not remove or weaken this locking.
- **Brace expansion cap** — `expand_items()` rejects expansions > 1M items to prevent OOM.
- **Filename sanitization** — `--results` uses a whitelist (`[a-zA-Z0-9._-]`), replaces everything else with `_`, collapses runs, truncates at 200 chars. Path traversal verified safe.
- **Memory ordering** — `Acquire`/`Release`/`AcqRel` on atomics for the halt flag and failure counter.

### Syntax Detection

The dual-syntax detection in `split_rest()` is the trickiest part of the codebase. Key rules:

1. If `rest[0]` contains `{}` or `{#}` → GNU-parallel mode (it's a template)
2. If `rest[1] == "in"` → bash-style with items
3. If `rest[1] == "--"` or `rest[1] == "do"` → bash-style with stdin
4. Otherwise → GNU-parallel fallback

**Inherent ambiguity**: The word "in" as `rest[1]` triggers bash detection. If a user's second positional in GNU-parallel mode happens to be "in", they'll get a bash-style parse error. This is an accepted tradeoff — the error message is actionable.

### Exit Codes

- `0` — all jobs succeeded
- `1-125` — count of failed jobs, capped at 125
- `2` — usage error (bad args, missing file, etc.)

### Feature Flags and Interactions

All flags compose cleanly:

| Combo | Behavior |
|-------|----------|
| `--dry-run` + anything | Prints commands, skips execution, exit 0 |
| `--group` + `-j N` | Buffers per-job output, flushes atomically |
| `--group` + `--retries` | Only final attempt's output shown |
| `--retries` + `--halt-on-fail` | Halt only after retries exhausted |
| `--results` + `--group` | Saved files match grouped output |
| `--results` + `--retries` | Only final attempt saved |
| `--multi-bar` + `-j 1` or non-TTY | Graceful fallback to single bar |

## Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }  # CLI parsing
indicatif = "0.17"                                   # Progress bars
crossbeam-channel = "0.5"                            # Job queue coordination

[dev-dependencies]
assert_cmd = "2"    # Black-box CLI testing
predicates = "3"    # Test assertions
tempfile = "3"      # Temp files for tests
```

Minimal dependency footprint. `num_cpus` was dropped in favor of `std::thread::available_parallelism()` (stable since Rust 1.59).

## Future Work

Deferred features (not in scope, do not implement without explicit approval):

- **Multiple named variables** — `rfor i j in (a,1) (b,2) -- echo {i} {j}`. Complex parsing, unclear syntax.
- **`-f FILE` / `--template-file FILE`** — Read template from a file. Useful for complex multi-line templates.
- **`{%}` job slot token** — GNU parallel compatibility.
- **`--retry-delay SECS`** — Backoff between retries.

## Project History

Built by an 8-agent AI development team (Architect, Implementer, Tester, Reviewer, Critique, Documenter, Instructor, Noob) across 6 sprints in ~10 hours. See `docs/index.html` for the interactive development timeline.

Originally named `pfor`, renamed to `rfor` (Rust for) after completion.
