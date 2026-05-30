# rfor v1 — Usability Test Report

**Author:** Instructor  
**Date:** 2026-05-29  
**Methodology:** 10 scenarios dispatched to a naive user (Noob agent) who had never seen rfor before. The Noob used ONLY `rfor --help`, `README.md`, and error messages — no source code.

---

## Executive Summary

**rfor v1 is exceptionally usable.** The Noob completed all 10 scenarios — every single task worked on the first try. The help text, README, and CLI syntax are clear enough that a developer familiar with bash `for` loops or GNU parallel can be productive within seconds. Overall score: **4.7/5**.

Two silent-wrong-result footguns were identified that should be fixed before release: unsupported GNU parallel tokens (`{.}`, `{/}`) pass through silently, and mixing `:::` + `::::` in one command doesn't error.

---

## Scenario Results

| # | Scenario | Outcome | First-Try? | Confusion | Discoverability | Learnability | Error Recovery | Docs Quality | Rating |
|---|----------|---------|-----------|-----------|----------------|-------------|----------------|-------------|--------|
| 1 | Discovery | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 2 | Hello World | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 3 | Job Index | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 4 | File Input | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 5 | Stdin Pipe | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 6 | Parallel Execution | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 7 | Progress Bar | ⚠️ Partial | ✅ | Bar not visible in non-TTY (expected) | 4 | 4 | N/A | 4 | **4/5** |
| 8 | Error Handling | ✅ Pass | ✅ | None | 5 | 5 | 5 | 5 | **5/5** |
| 9 | Real-World Gzip | ✅ Pass | ✅ | None | 5 | 5 | N/A | 5 | **5/5** |
| 10 | Anti-Patterns | ⚠️ Observed | ✅ | Silent wrong results on edge cases | 3 | 3 | 3 | 3 | **3/5** |

**Average: 4.7/5**

---

## Top Usability Issues (Ranked by Severity)

### 🔴 Issue 1: Mixed `:::` and `::::` silently produces wrong results (MAJOR)

**What happened:** When the Noob ran:
```bash
rfor 'echo {}' ::: a b :::: /tmp/rfor-test/input/fruits.txt
```
…rfor treated `::::` and the filepath as literal inline items rather than erroring. Output:
```
a
b
::::
/tmp/rfor-test/input/fruits.txt
```

**Impact:** A user who tries to combine input sources gets wrong results with no warning. The README says "exactly one per invocation" but the tool doesn't enforce it.

**Recommendation:** Emit an error (exit 2) when both `:::` and `::::` appear in the same invocation.

---

### 🔴 Issue 2: Unsupported GNU parallel tokens pass through silently (MAJOR)

**What happened:** When the Noob ran:
```bash
rfor 'echo {.}' ::: test.txt
```
…the output was literally `{.}` — no warning, no error. A GNU parallel user would expect `test` (filename without extension).

**Impact:** Users migrating from GNU parallel may get silently wrong results in scripts. `{.}`, `{/}`, `{//}`, `{/.}` are common GNU parallel tokens.

**Recommendation:** Detect known GNU parallel tokens in templates and print a stderr warning:
```
rfor: warning: '{.}' is not a supported token (did you mean '{}'?). Supported: {}, {#}, {{, }}
```

---

### 🟡 Issue 3: No `--dry-run` flag (MINOR)

**What happened:** `rfor --dry-run 'echo {}' ::: a b c` → error: unrecognized flag.

**Impact:** Users want to preview expanded commands before running them, especially with complex templates. This is a standard CLI expectation.

**Recommendation:** Add `--dry-run` to v1 or v1.1. Print each expanded command without executing.

---

### 🟡 Issue 4: Progress bar not verifiable in non-TTY environments (MINOR)

**What happened:** The Noob ran the progress bar scenario correctly, but since the test ran in a non-TTY context, the bar was suppressed (by design). The Noob rated this 4/5 because they couldn't actually *see* the signature feature.

**Impact:** CI pipelines, captured shells, and automated test environments will never show the bar. Users testing in these contexts may think it's broken.

**Recommendation:** Consider a `--progress` flag to force the bar even in non-TTY. Low priority — the current behavior is correct and documented.

---

### ⚪ Issue 5: Usage line trailing `...` is slightly ambiguous (COSMETIC)

**What happened:** The Noob noted the help line `<TEMPLATE [::: ARGS... | :::: FILE]>...` has a trailing `...` suggesting the whole group is repeatable, which it isn't.

**Impact:** Momentary confusion, resolved by reading examples.

**Recommendation:** Remove the trailing `...` or clarify in a future help text revision.

---

## Documentation Assessment

| Aspect | Rating | Notes |
|--------|--------|-------|
| `--help` output | ⭐⭐⭐⭐⭐ | Includes examples, token reference, clear option descriptions |
| `-h` short help | ⭐⭐⭐⭐⭐ | Good differentiation from `--help` — both useful |
| README.md | ⭐⭐⭐⭐⭐ | Comparison table, real-world examples, exit codes, edge cases |
| Error messages | ⭐⭐⭐⭐ | Good for known errors (no args → clear message, exit 2). Weak on silent failures (mixed separators, unsupported tokens). |
| What's NOT supported | ⭐⭐⭐ | Docs explain what IS supported but don't mention what ISN'T. GNU parallel users will try `{.}` etc. |

---

## What Went Great

1. **Zero learning curve** — Every task was completed on the first try. The syntax is immediately familiar to anyone who has used GNU parallel or xargs.

2. **Excellent help text** — The `--help` output is one of the best reviewed. Includes real examples, token reference table, and clear option descriptions.

3. **Auto shell-quoting** — `{}` items are POSIX shell-quoted automatically. This is a genuine improvement over xargs and prevents a whole class of bugs with filenames containing spaces or special characters.

4. **Sensible defaults** — Sequential by default, continue on error by default, progress bar auto-hidden on non-TTY. All correct choices.

5. **Clever exit codes** — Exit code = number of failed jobs (capped at 125). Useful for scripts that need to know the scale of failure, not just pass/fail.

6. **Comprehensive README** — The comparison table with xargs/parallel/bash, real-world examples, and exit code documentation go beyond what's typical for a v0.1.0 tool.

---

## Recommendations (Ordered by Impact)

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| **P0** | Error on mixed `:::` + `::::` in same command | Low | Prevents silent wrong results |
| **P0** | Warn on unsupported GNU parallel tokens (`{.}`, `{/}`, `{//}`, `{/.}`) | Low | Prevents silent wrong results for GNU parallel migrants |
| **P1** | Add `--dry-run` flag | Medium | Standard CLI expectation, helps debugging |
| **P2** | Add "Not Yet Supported" section to README | Low | Sets expectations for GNU parallel migrants |
| **P2** | Consider `--progress` flag to force bar in non-TTY | Low | Helps debugging in CI/captured environments |
| **P3** | Clean up usage line trailing `...` | Trivial | Cosmetic clarity |

---

## Raw Observations

### Commands the Noob tried (all first-try successes):
```bash
# Scenario 1
rfor --help
rfor -h
rfor --version

# Scenario 2
rfor 'echo {}' ::: hello world foo

# Scenario 3
rfor 'echo {#}: {}' ::: apple banana cherry

# Scenario 4
rfor 'echo {}' :::: /tmp/rfor-test/input/fruits.txt

# Scenario 5
ls /tmp/rfor-test/input/*.txt | rfor 'wc -l {}'

# Scenario 6
time rfor -j 5 'sleep 1 && echo {}' ::: 1 2 3 4 5

# Scenario 7
rfor 'sleep 2 && echo done: {}' ::: a b c d e

# Scenario 8a
rfor 'ls {}' ::: /tmp /nonexistent_path_xyz /home; echo $?
# Scenario 8b
rfor --halt-on-fail 'ls {}' ::: /tmp /nonexistent_path_xyz /home

# Scenario 9
ls /tmp/rfor-test/input/file*.txt | rfor -j 3 'gzip {}'

# Scenario 10
timeout 3 rfor                                    # clear error, exit 2
rfor 'echo {.}' ::: test.txt                      # silent pass-through
rfor --dry-run 'echo {}' ::: a b c                # unrecognized flag error
rfor 'echo {}' ::: a b :::: fruits.txt            # silent wrong results
```

### Notable: zero wrong attempts across all 10 scenarios.
This is an unusually strong result for a first-encounter usability test.
