# rfor v1 Usability Test Plan

**Author:** Instructor agent  
**Date:** 2026-05-29  
**Status:** Pre-draft (awaiting implementation + docs before dispatch)

---

## Testing Philosophy

The Noob agent simulates a developer who:
- Has used bash `for` loops and maybe heard of GNU parallel
- Has never seen `rfor` before
- Will rely **only** on `rfor --help` and the project README to figure things out
- Will type commands exactly as they understand them (mistakes are signal, not noise)

**What we're testing:** Can a first-time user accomplish real tasks using only the docs?  
**What we're NOT testing:** Internals, edge-case correctness (that's the Tester's job).

---

## Test Environment Setup

Before dispatching to the Noob, ensure:
1. `rfor` binary is built and on PATH (or provide the exact path)
2. Create a scratch directory with test fixtures:

```bash
mkdir -p /tmp/rfor-test/input
echo -e "apple\nbanana\ncherry\ndate\nelderberry" > /tmp/rfor-test/input/fruits.txt
for i in 1 2 3 4 5; do echo "line $i" > /tmp/rfor-test/input/file$i.txt; done
```

---

## Scenarios

### Scenario 1: Discovery — "What is this tool?"

**Goal:** User learns what rfor does and how to use it.  
**Motivation:** Every CLI journey starts with `--help`.

**Task for Noob:**
> You just installed a tool called `rfor`. Figure out what it does and what flags are available. Then tell me the basic syntax.

**Expected approach:**
- Run `rfor --help` (or `rfor -h`)
- Run `rfor --version` (or `rfor -V`)

**Success criteria:**
- [ ] `--help` output is clear enough that the Noob can describe the syntax
- [ ] The Noob identifies `:::`, `::::`, and stdin as input methods
- [ ] The Noob identifies `{}` and `{#}` tokens
- [ ] The Noob identifies `-j` and `--halt-on-fail` flags

**Usability signals to watch:**
- Does the Noob try `rfor` with no args first? What happens?
- Is the help text scannable or overwhelming?
- Does the Noob confuse `:::` with `::::` ?

---

### Scenario 2: Hello World — "Run a command for each item"

**Goal:** Replace a simple `for i in a b c; do echo $i; done` with rfor.  
**Motivation:** The #1 use case — the reason the tool exists.

**Task for Noob:**
> Using rfor, echo each of the words "hello", "world", "foo" on its own line. The equivalent bash would be: `for i in hello world foo; do echo $i; done`

**Expected command:**
```bash
rfor 'echo {}' ::: hello world foo
```

**Success criteria:**
- [ ] The Noob produces correct output (hello, world, foo each on a line)
- [ ] The Noob uses `{}` correctly as the placeholder
- [ ] The command is constructed on the first or second try

**Usability signals to watch:**
- Does the Noob quote the command template? (Single vs double quotes)
- Does the Noob forget `:::` or put it in the wrong place?
- Does the Noob try bash-style `$i` instead of `{}`?

---

### Scenario 3: Job Index — "Number my outputs"

**Goal:** Use the `{#}` token to get a 1-based index.  
**Motivation:** Users often want numbered output (e.g., "Processing item 3 of 5").

**Task for Noob:**
> Using rfor, print a numbered list like:
> ```
> 1: apple
> 2: banana
> 3: cherry
> ```
> Use the words apple, banana, cherry as input.

**Expected command:**
```bash
rfor 'echo {#}: {}' ::: apple banana cherry
```

**Success criteria:**
- [ ] Output shows 1-based numbering with the correct items
- [ ] The Noob discovers `{#}` from docs without hints

**Usability signals to watch:**
- Does the Noob find `{#}` in the help text easily?
- Does the Noob try `{0}`, `{i}`, `{n}`, or other index guesses first?
- Quote handling: does `{#}` get eaten by the shell? (It shouldn't in single quotes)

---

### Scenario 4: File Input — "Read args from a file"

**Goal:** Use `::::` to read arguments from a file.  
**Motivation:** Real workloads come from files (server lists, URLs, etc.).

**Task for Noob:**
> You have a file at `/tmp/rfor-test/input/fruits.txt` with one fruit per line. Using rfor, run `echo {}` for each fruit in that file.

**Expected command:**
```bash
rfor 'echo {}' :::: /tmp/rfor-test/input/fruits.txt
```

**Success criteria:**
- [ ] The Noob reads all 5 fruits from the file
- [ ] The Noob uses `::::` (not `:::`)

**Usability signals to watch:**
- Does the Noob confuse `:::` and `::::` ?
- Does the Noob try `cat file | rfor ...` instead? (That's valid too — note it)
- What error does the Noob get if they use the wrong one?

---

### Scenario 5: Stdin Pipe — "Pipe data into rfor"

**Goal:** Use stdin as the argument source.  
**Motivation:** Unix philosophy — piping is second nature.

**Task for Noob:**
> List the .txt files in `/tmp/rfor-test/input/` and pipe them to rfor to count lines in each file with `wc -l`.

**Expected command:**
```bash
ls /tmp/rfor-test/input/*.txt | rfor 'wc -l {}'
```

**Success criteria:**
- [ ] The Noob successfully pipes input to rfor
- [ ] Output shows line counts for each file

**Usability signals to watch:**
- Does the Noob omit `{}` and expect implicit substitution?
- Does the Noob try to add `:::` or `::::` alongside the pipe?
- Does the Noob understand that stdin replaces the `:::` arg list?

---

### Scenario 6: Parallel Execution — "Go faster"

**Goal:** Use `-j` to run jobs in parallel.  
**Motivation:** The whole point of having concurrency control.

**Task for Noob:**
> Run `sleep 1 && echo {}` for the numbers 1 through 5, but run them all in parallel so the total time is ~1 second, not ~5 seconds.

**Expected command:**
```bash
rfor -j 5 'sleep 1 && echo {}' ::: 1 2 3 4 5
```
(or `-j 0` for all CPUs)

**Success criteria:**
- [ ] Total wall time is ~1 second (not 5)
- [ ] All 5 items are echoed
- [ ] The Noob understands `-j` from the docs

**Usability signals to watch:**
- Does the Noob discover `-j 0` means "use all CPUs"?
- Does the Noob place `-j` in the right position (before the command template)?
- Does the Noob try `-j` without a value?

---

### Scenario 7: Progress Bar — "I want to see progress"

**Goal:** Observe the progress bar during a longer-running task.  
**Motivation:** The signature feature of rfor.

**Task for Noob:**
> Run a slow command (`sleep 2 && echo done: {}`) for items a through e, sequentially (default). Watch for a progress bar at the bottom of the terminal.

**Expected command:**
```bash
rfor 'sleep 2 && echo done: {}' ::: a b c d e
```

**Success criteria:**
- [ ] Progress bar appears at bottom of terminal
- [ ] Bar shows count and/or ETA
- [ ] Command output streams above the bar
- [ ] Bar disappears cleanly when done

**Usability signals to watch:**
- Does the Noob notice the progress bar?
- Does output interleave with the bar or stay clean?
- If the Noob pipes output (`| tee log.txt`), does the bar vanish? (It should — non-TTY)

---

### Scenario 8: Error Handling — "What if a command fails?"

**Goal:** Understand default behavior when a job fails, and the effect of `--halt-on-fail`.  
**Motivation:** Real commands fail. Users need predictable behavior.

**Task for Noob (Part A — default continue):**
> Run `ls {}` for these paths: `/tmp`, `/nonexistent`, `/home`. What happens? Does rfor stop or continue? What exit code do you get?

**Expected command:**
```bash
rfor 'ls {}' ::: /tmp /nonexistent /home
echo $?
```

**Expected behavior:** rfor continues past the failing `ls /nonexistent`, runs all 3 jobs, exits with code 1 (1 failure, capped at 125).

**Task for Noob (Part B — halt-on-fail):**
> Now run the same command but add `--halt-on-fail`. Does it stop after the failure?

**Expected command:**
```bash
rfor --halt-on-fail 'ls {}' ::: /tmp /nonexistent /home
```

**Success criteria:**
- [ ] Part A: Noob observes that rfor continues past failure by default
- [ ] Part A: Noob checks and reports the exit code
- [ ] Part B: Noob successfully uses `--halt-on-fail`
- [ ] Part B: Noob observes that rfor stops after the first failure

**Usability signals to watch:**
- Is "halt-on-fail" discoverable? Does the Noob try `--stop-on-error` or similar?
- Is the exit code behavior explained in `--help`?
- Does the error message from the failed job show clearly?

---

### Scenario 9: Real-World Task — "Compress some files"

**Goal:** A realistic task combining multiple features.  
**Motivation:** The payoff scenario — does everything come together?

**Task for Noob:**
> You have 5 text files in `/tmp/rfor-test/input/` (file1.txt through file5.txt). Using rfor, gzip each one in parallel using 3 workers. Show progress.

**Expected command:**
```bash
rfor -j 3 'gzip {}' ::: /tmp/rfor-test/input/file*.txt
```
(or `ls /tmp/rfor-test/input/file*.txt | rfor -j 3 'gzip {}'`)

**Success criteria:**
- [ ] All 5 files get compressed
- [ ] Runs with 3 parallel workers
- [ ] Progress bar shows during execution
- [ ] Noob constructs the command without hand-holding

**Usability signals to watch:**
- Does the Noob try glob expansion inside `:::`? Does that work?
- Does the Noob combine `-j` and progress bar correctly?
- Any confusion about argument ordering?

---

### Scenario 10: Anti-Pattern Discovery — "What DOESN'T work?"

**Goal:** Probe the edges of v1 to see if docs set expectations correctly.  
**Motivation:** Users will try things outside scope. Good docs prevent frustration.

**Task for Noob:**
> Try the following and note what happens:
> 1. Run rfor with no arguments at all
> 2. Try to use `{.}` (filename without extension) as a token
> 3. Try to use `--dry-run` to preview commands
> 4. Try to combine `:::` and `::::` in one command

**Success criteria:**
- [ ] `rfor` with no args shows help or a clear error
- [ ] `{.}` is not expanded (passed literally) — docs should mention this is not yet supported
- [ ] `--dry-run` is rejected with a clear error, not silently ignored
- [ ] Combined `:::` + `::::` either works sensibly or fails with a clear message

**Usability signals to watch:**
- Are error messages helpful or cryptic?
- Do the docs mention what's NOT supported (managing expectations)?
- Does the Noob get frustrated or confused at any point?

---

## Scoring Rubric

After the Noob completes all scenarios, score each on:

| Dimension | 1 (Fail) | 2 (Struggle) | 3 (Okay) | 4 (Smooth) | 5 (Delight) |
|-----------|----------|---------------|-----------|-------------|--------------|
| **Discoverability** | Couldn't find the feature | Found after many wrong guesses | Found via help after a couple tries | Found on first try via help | Intuitive, didn't even need help |
| **Learnability** | Couldn't complete the task | Completed with major confusion | Completed with minor confusion | Completed cleanly | Completed and extrapolated to new uses |
| **Error Recovery** | Error was cryptic/unhelpful | Error pointed wrong direction | Error was understandable | Error suggested the fix | Error + fix was automatic |
| **Docs Quality** | Docs missing or wrong | Docs confusing | Docs adequate | Docs clear | Docs excellent with examples |

---

## UX Report Template

After all scenarios are complete, produce a report with:

1. **Executive Summary** — 2-3 sentence overall verdict
2. **Scenario Results Table** — Pass/Fail/Partial for each scenario with scores
3. **Top Usability Issues** — Ranked by severity (blocker / major / minor / cosmetic)
4. **Documentation Gaps** — Specific things missing or unclear in docs
5. **Recommendations** — Concrete fixes, ordered by impact
6. **Raw Observations** — Noob's exact commands, errors, and confusion points
