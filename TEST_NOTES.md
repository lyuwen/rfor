# pfor test suite — notes for architect, reviewer, and implementer

## What this branch contains
All files under `tests/`, plus a `Cargo.toml` containing **only** a `[package]`
stub and `[dev-dependencies]`. The implementer's branch will contain
`[dependencies]`, `[[bin]]` (if needed), and the actual source. **Expect a
predictable merge conflict in `Cargo.toml` at merge time — the resolution is
the UNION of the two files.**

## Black-box test strategy
Tests invoke the `pfor` binary via `assert_cmd::Command::cargo_bin("pfor")`.
No test imports anything from the production crate. This was a deliberate
constraint set by the architect.

## Assumptions about pfor's runtime
The spec doesn't pin how `pfor` invokes the command template (e.g. `sh -c
<template>` vs internal argv splitting). The tests assume **`sh -c`-style**
execution because:
1. It matches GNU parallel's conventional behavior, which the spec invokes
   stylistically.
2. Most templates the tests use are written to work either way — they call
   `sh -c '...'` explicitly inside the template when shell features (`;`, `$0`,
   `>>`) are needed.

If the implementer chose a substantially different execution model, several
tests will fail and the architect should mediate.

## Ambiguities the tester resolved (please confirm)
- **`:::` vs stdin precedence**: tests assume `:::` wins (spec implies it).
  Test: `sources::triple_colon_wins_over_stdin`.
- **`{{` / `}}` escaping**: task description listed this as testable; spec
  is silent. Test: `template::double_braces_escape_to_literal_braces`.
- **Argfile blank lines**: spec silent. Test accepts EITHER skip-blank OR
  treat-as-empty-item behavior. Test: `sources::argfile_blank_lines_are_handled`.
- **Exit code cap (125)**: from the task; spec just says "capped at 125".
  Test: `exit_codes::failure_count_caps_at_125`.

## Coverage map (task bullets → tests)
- Template substitution `{}` → `template::item_placeholder_basic`,
  `template::placeholder_with_special_characters_in_item`
- Template substitution `{#}` → `template::job_index_placeholder_is_one_based`
- Escape `{{` / `}}` → `template::double_braces_escape_to_literal_braces`
- `:::` source → `sources::triple_colon_introduces_literal_args`
- `::::` argfile → `sources::quadruple_colon_reads_items_from_file`,
  `sources::empty_argfile_produces_zero_jobs_and_exits_zero`
- stdin source → `sources::stdin_one_item_per_line`,
  `sources::stdin_without_trailing_newline_is_still_a_job`
- Empty input → `sources::empty_stdin_produces_zero_jobs_and_exits_zero`
- `--help` / `--version` exit 0 → `cli::*`
- Sequential ordering → `sequential::*`
- Parallel throughput → `parallel::parallel_is_faster_than_sequential_on_sleeping_jobs`
- `-j 0` = ncpu → `parallel::jobs_zero_means_num_cpus_and_runs_in_parallel`
- Per-line atomicity → `parallel::per_line_output_is_atomic_under_parallel`
- Off-TTY: no ANSI → `tty::*`
- Exit code (all succeed / partial fail / cap) → `exit_codes::*`
- `--halt-on-fail` → `halt_on_fail::*`
- Live streaming → `streaming::*`
- Missing template → `cli::missing_command_template_is_an_error`
- `:::` + stdin precedence → `sources::triple_colon_wins_over_stdin`
- Template with no placeholder → `template::template_without_placeholder_runs_per_item`

## Not tested (manual / out of scope)
- **Interactive TTY rendering of the indicatif sticky progress bar.** Requires
  a real PTY; not worth a `portable-pty` dependency for v1. Manual verification
  needed:
  - Run `pfor -j 4 'sleep 1; echo {}' ::: a b c d e f g h` from a real
    terminal. Observe: a bar pinned at the bottom showing `N/8` and ETA, with
    job output streaming above it; on completion the bar clears or finalises
    cleanly.
- Out-of-scope tokens (`{.}` `{/}` `{//}`), `--results`, `--retries`,
  `--group`, `--dry-run`, multi-bar mode, shell-style `for i in ...` syntax —
  per spec.

## Why tests use generous time margins
Throughput and streaming tests check wall-clock behavior. CI hosts and shared
runners are noisy; margins are intentionally loose (≥3.5s lower bound for
sequential 8×0.5s, ≤2.5s upper bound for parallel) so that the suite is robust
on slow machines but still catches regression-grade failures.

## Files
```
Cargo.toml                 # [package] + [dev-dependencies] only
tests/common/mod.rs        # shared helpers
tests/cli.rs               # help / version / argument errors
tests/template.rs          # {} / {#} / escaping
tests/sources.rs           # ::: / :::: / stdin / precedence
tests/sequential.rs        # -j 1 ordering
tests/parallel.rs          # -j N throughput, atomicity, -j 0
tests/exit_codes.rs        # 0 / K / 125 cap
tests/halt_on_fail.rs      # --halt-on-fail sentinel-file proof
tests/streaming.rs         # live-output timing (raw std::process)
tests/tty.rs               # off-TTY: no ANSI leaks
TEST_NOTES.md              # this file
```
