# rfor v1 scope (user-approved 2026-05-29)

**Binary name:** `rfor`

**Syntax (GNU-parallel-style):**
```
rfor [flags] '<command template>' ::: arg1 arg2 ...
rfor [flags] '<command template>' :::: argfile
<stdin> | rfor [flags] '<command template>'
```

**Tokens:** `{}` (item), `{#}` (1-based job index). Defer `{.}`, `{/}`, `{//}`.

**Concurrency:** sequential default (`-j 1`). `-j N` for parallel. `-j 0` = num CPUs.

**Progress bar:** sticky at bottom on TTY (via `indicatif`). Off-TTY = no bar, plain pass-through. Bar shows count + ETA.

**Flags (v1):**
- `-j, --jobs N` (default 1)
- `--halt-on-fail` (default: continue like GNU parallel)
- `-h, --help`, `-V, --version`

**Behavior:**
- Exit code: 0 if all jobs succeeded, non-zero (count of failed jobs, capped at 125) otherwise.
- Child stdout+stderr stream live above the bar.
- TTY detection: stderr fd via `std::io::IsTerminal`.

**Out of scope for v1 (do not implement, do not test):**
`{.}`/`{/}`/`{//}` filename tokens, `--results` directory, `--retries`, multi-bar per-job display, `--group` output buffering, `--dry-run`, shell-style `for i in ...; do ...; done` syntax.

**Branching:**
- `dev/rfor` — delivery branch, PR target into main
- `feat/rfor-core` — implementer worktree
- `test/rfor-core` — tester worktree
