---
name: dev-rfor-bash-syntax-scope
description: rfor bash for-loop syntax feature — scope, syntax, parsing, named variables (user-approved 2026-05-29)
metadata:
  type: project
---

# rfor bash for-loop syntax (user-approved 2026-05-29)

**Feature:** Add bash-style `rfor VAR in ITEMS -- COMMAND` syntax with real named variables, coexisting with GNU parallel syntax.

**Syntax:**
```bash
# Bash-style with inline items
rfor i in a b c -- echo {i}

# Bash-style with file
rfor i in :::: urls.txt -- curl -sO {i}

# Bash-style with stdin
cat items.txt | rfor i -- echo {i}

# With flags
rfor -j 4 i in a b c -- echo {i}
rfor --halt-on-fail i in a b c -- echo {i}

# GNU parallel style (unchanged, still works)
rfor 'echo {}' ::: a b c
```

**Named variables (real, not cosmetic):**
- `{varname}` only substitutes if `varname` matches the declared variable
- `{other}` passes through literally when var is `i`
- `{}` always works (unnamed placeholder, backward compat)
- `{#}` always works (job index, backward compat)

**Detection logic in `split_rest()`:**
- If `rest[0]` contains `{}` or `{#}` → GNU-parallel (template detected)
- Else if `rest[1] == "in"` → bash-style with items
- Else if `rest[1] == "--"` → bash-style with stdin
- Else → GNU-parallel style

**Parsing — bash-style with items:**
- `rest[0]` = var_name
- `rest[1]` = "in" (skip)
- `rest[2..sep]` = items (where sep = position of "--")
- `rest[sep+1..]` = command words, joined with spaces
- If items are `["::::", "file.txt"]` → treat as argfile
- Error if no `--` found
- Error if no command words after `--`

**Parsing — bash-style with stdin:**
- `rest[0]` = var_name
- `rest[1]` = "--" (skip)
- `rest[2..]` = command words, joined with spaces

**Files to modify:** cli.rs (SplitArgs + parsing), template.rs (render + warn), runner.rs (RunConfig), main.rs (wiring), help text, README.md

**New tests:** `tests/bash_style.rs` — 12 scenarios covering basic usage, named var matching, wrong name passthrough, `{}` and `{#}` compat, stdin, argfile, flags, error cases.

**Branching:**
- `dev/rfor-bash-syntax` from main
- `feat/rfor-bash-syntax` for implementer
- `test/rfor-bash-syntax` for tester

**Out of scope:**
- Multiple named variables
- Brace expansion
- `do`/`done` keywords
- Implicit append in bash-style (must use `{varname}` or `{}`)

**Related:** [[dev-rfor-scope]] [[dev-language-stack]]
