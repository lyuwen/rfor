---
name: dev-pfor-roadmap
description: Prioritized feature roadmap for pfor — 9 features across 4 sprints (user-approved 2026-05-29)
metadata:
  type: project
---

# pfor Feature Roadmap (2026-05-29)

All features below are deferred from v1 and bash-syntax. Ranked by user value.

## Sprint 1 — Quick Wins (High Impact, Low Effort)

### 1. `--dry-run` ⭐⭐⭐⭐⭐
Print rendered commands without executing. P1 usability finding.
- Add `--dry-run` flag to clap
- In runner: print rendered command to stdout, skip spawn
- Still show progress bar (count of "processed" items)

### 2. Filename Tokens ⭐⭐⭐⭐⭐
`{.}` (strip extension), `{/}` (basename), `{//}` (dirname), `{/.}` (basename without extension).
- All four are string manipulation in template.rs
- Ship as one unit — they're always used together
- Example: `pfor f in *.png -- convert {f} {.}.jpg`

## Sprint 2 — Daily Driver (High Impact, Medium Effort)

### 3. `--group` ⭐⭐⭐⭐
Buffer output per job — no interleaving in parallel mode.
- Each job's stdout/stderr collected in memory
- Flushed as a block when job completes

### 4. `--retries N` ⭐⭐⭐⭐
Retry failed jobs up to N times.
- Retry loop around job execution in runner
- Optional `--retry-delay SECS` for backoff

## Sprint 3 — Automation (Medium Impact, Medium Effort)

### 5. `--results DIR` ⭐⭐⭐
Save each job's stdout/stderr to files in DIR.

### 6. Brace Expansion ⭐⭐⭐
`{1..10}`, `{a..z}`, `{01..99}` ranges.

## Sprint 4 — Polish (Lower Impact, Higher Effort)

### 7. `do`/`done` Keywords (Experimental) ⭐⭐⭐
True bash-like syntax: `pfor i in a b c do echo {i} done`
- NOT blocked by shell parsing — `do`/`done` are regular strings as args to pfor
- Implementation: accept `do` as alias for `--`, strip trailing `done`

### 8. Multi-bar Per-Job Display ⭐⭐
Individual progress bars per running job.

### 9. Multiple Named Variables ⭐⭐
`pfor i j in (a,1) (b,2) -- echo {i} {j}`

## Dependencies
- `--results DIR` (#5) benefits from filename tokens (#2)
- All other features are independent

**Related:** [[dev-pfor-scope]] [[dev-pfor-bash-syntax-scope]] [[dev-language-stack]]
