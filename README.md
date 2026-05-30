# rfor

A parallel for-loop replacement with live output and a sticky progress bar.

`rfor` runs a shell command once per item — from inline arguments, a file, or
stdin — with an optional progress bar pinned to the bottom of your terminal.
Think of it as a focused alternative to `xargs` or GNU `parallel` for
day-to-day loops that benefit from visual feedback.

## Features

- **Live output** — child stdout and stderr stream above the progress bar in
  real time, never interleaved at the line level.
- **Sticky progress bar** — count, elapsed time, and ETA stay pinned to the
  bottom row on TTY terminals.  Automatically hidden when output is piped or
  redirected.
- **Parallel or sequential** — sequential by default (`-j 1`); scale to any
  concurrency with `-j N` or let the system choose with `-j 0`.
- **Two syntax styles** — GNU-parallel-style (`:::` / `::::`) *and*
  bash-for-loop-style (`rfor i in ... -- command {i}`).  Pick whichever
  reads more naturally.
- **Named variables** — bash-style syntax lets you name the loop variable
  (`{url}`, `{file}`, `{host}`) so commands read like documentation.

## Installation

### From source (requires [Rust ≥ 1.70](https://rustup.rs))

```sh
git clone https://github.com/<owner>/rfor.git
cd rfor
cargo install --path .
```

The binary lands in `~/.cargo/bin/rfor`.  Make sure that directory is on your
`PATH`.

### Build a release binary without installing

```sh
cargo build --release
# binary is at ./target/release/rfor
```

## Quick start

```sh
# Greet three items inline (GNU parallel style)
rfor 'echo hello {}' ::: world rust rfor

# Same thing, bash for-loop style
rfor name in world rust rfor -- echo hello {name}

# Download URLs listed in a file, 4 at a time
rfor -j 4 'curl -sO {}' :::: urls.txt

# Pipe items from another command
find . -name '*.log' | rfor 'gzip {}'
```

## Usage

`rfor` supports two syntax styles.  They are fully interchangeable — pick
whichever is clearer for your use case.

### GNU parallel style

```
rfor [OPTIONS] '<command template>' ::: item1 item2 ...
rfor [OPTIONS] '<command template>' :::: argfile
<stdin> | rfor [OPTIONS] '<command template>'
```

The first positional argument is the **command template** — a shell string
executed via `sh -c` for each item.

### Bash for-loop style

```
rfor [OPTIONS] VAR in item1 item2 ... -- command {VAR}
rfor [OPTIONS] VAR in :::: argfile   -- command {VAR}
<stdin> | rfor [OPTIONS] VAR         -- command {VAR}
```

`VAR` is a variable name you choose (e.g. `i`, `url`, `host`).  Everything
between `in` and `--` becomes the item list.  Everything after `--` is the
command, with `{VAR}` expanded to the current item.

The `--` separator is required — it tells `rfor` where items end and the
command begins.

> **How rfor picks the style:** if the first positional contains `{}` or
> `{#}`, it's treated as a GNU-parallel template.  Otherwise, if the second
> word is `in` or `--`, it's bash-style.  Everything else falls back to
> GNU-parallel.

### Item sources

Items come from one of three sources (exactly one per invocation):

| Source | GNU parallel style | Bash for-loop style |
|--------|-------------------|---------------------|
| Inline | `rfor 'echo {}' ::: a b c` | `rfor i in a b c -- echo {i}` |
| File | `rfor 'echo {}' :::: items.txt` | `rfor i in :::: items.txt -- echo {i}` |
| Stdin | `cat list \| rfor 'echo {}'` | `cat list \| rfor i -- echo {i}` |

Blank lines in files and stdin are silently skipped.

> **Tip:** if you run `rfor` without `:::`, `::::`, or piped input, it reads
> from the terminal and prints a hint.  Press **Ctrl-D** to finish, or
> re-run with an explicit item source.

## Template tokens

Tokens inside the command template are expanded before each invocation:

| Token | Expands to | Works in |
|-------|------------|----------|
| `{}`  | Current item (automatically shell-quoted) | Both styles |
| `{#}` | 1-based job index | Both styles |
| `{VAR}` | Current item, if `VAR` matches the declared variable name | Bash-style only |
| `{{`  | Literal `{` | Both styles |
| `}}`  | Literal `}` | Both styles |

Items substituted via `{}` or `{VAR}` are POSIX-shell-quoted (single-quoted
with embedded `'` escaped), so filenames with spaces, quotes, or glob
characters are handled safely.

### Named variables (bash-style)

In bash-style syntax, `{VAR}` is only substituted when `VAR` matches the
declared variable name.  Other names pass through literally:

```sh
rfor i in a b c -- echo {i} and {other}
# Output:
#   a and {other}
#   b and {other}
#   c and {other}
```

This is intentional — it means commands containing literal braces (like
`jq '{.name}'`) work without escaping, as long as the content inside the
braces doesn't match your variable name.

You can use `{}`, `{#}`, and `{VAR}` together in the same command:

```sh
rfor i in a b c -- echo job {#}: {} is {i}
# Output:
#   job 1: a is a
#   job 2: b is b
#   job 3: c is c
```

Variable names must be valid identifiers: start with a letter or underscore,
contain only letters, digits, and underscores.

## Options

| Flag | Long | Default | Description |
|------|------|---------|-------------|
| `-j` | `--jobs N` | `1` | Number of parallel workers.  `0` = number of logical CPUs. |
| | `--halt-on-fail` | off | Stop scheduling new jobs after the first failure.  Jobs already running will finish. |
| `-h` | `--help` | | Print help and examples. |
| `-V` | `--version` | | Print version (`rfor 0.1.0`). |

### Concurrency (`-j`)

```sh
# Sequential (default) — one job at a time
rfor 'make -C {}' ::: proj_a proj_b proj_c

# 4 parallel workers
rfor -j 4 'convert {} {}.webp' :::: images.txt

# As many workers as CPU cores
rfor -j 0 'cargo test -p {}' ::: core api web
```

With `-j 1` (the default), jobs run in order and output appears exactly as it
would in a `for` loop.  With `-j N` where N > 1, up to N jobs run
concurrently; output lines from different jobs may interleave, but individual
lines are never torn.

### Halt on failure (`--halt-on-fail`)

```sh
rfor --halt-on-fail 'deploy {}' ::: staging production
```

Without `--halt-on-fail` (the default), all items run even if some fail —
matching the behavior of GNU `parallel`.  When `--halt-on-fail` is active,
no *new* jobs are started after a failure, but any in-flight jobs are allowed
to finish.

## Progress bar

On a TTY terminal, `rfor` displays a sticky progress bar on the bottom line:

```
[========>-------------------------------] 3/10 [00:00:04] eta 00:00:12
```

The bar shows:

- Jobs completed out of total
- Elapsed time
- Estimated time remaining (ETA)

Child output streams live above the bar — the bar redraws after each line so
it always stays at the bottom.

When stderr is not a TTY (e.g. piped to a file or another process), the
progress bar is suppressed and output passes through unchanged.  This means
`rfor` is safe to use in pipelines:

```sh
rfor 'process {}' :::: items.txt 2>/dev/null   # bar hidden, stdout clean
rfor 'process {}' :::: items.txt | tee out.log  # bar hidden automatically
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | All jobs succeeded (exit 0). |
| `1`–`125` | Number of failed jobs (capped at 125). |
| `2`  | Usage error (bad arguments, missing template, unreadable argfile). |

Examples:

```sh
rfor 'true' ::: a b c
echo $?   # 0

rfor 'false' ::: a b c
echo $?   # 3  (all three failed)

rfor 'sh -c "[ {} = b ] && exit 1 || true"' ::: a b c
echo $?   # 1  (one failure)
```

## Real-world examples

### Compress log files in parallel

```sh
find /var/log -name '*.log' -mtime +7 | rfor -j 4 'gzip -9 {}'
```

### Run tests across multiple packages

```sh
rfor -j 0 pkg in core api web -- cargo test -p {pkg}
```

### Batch image conversion

```sh
ls *.png | rfor -j 8 img -- convert {img} -resize 800x600 resized/{img}
```

### Deploy to multiple hosts, stop on first failure

```sh
rfor --halt-on-fail host in web1 web2 web3 -- ssh {host} sudo systemctl restart myapp
```

### Download URLs from a file

```sh
rfor -j 4 url in :::: urls.txt -- curl -sfSL -o /tmp/{#}.html {url}
```

### Sequential build with progress tracking

```sh
rfor dir in lib1 lib2 lib3 app -- make -C {dir}
```

### Restart services across environments

```sh
rfor svc in nginx postgres redis -- sudo systemctl restart {svc}
```

## Comparison with other tools

| | `rfor` | `xargs -P` | GNU `parallel` | bash `for` loop |
|---|---|---|---|---|
| Progress bar | ✅ Sticky ETA bar | ❌ | ❌ (requires `--bar`) | ❌ |
| Live output | ✅ Streams above bar | ✅ (may interleave) | ⚠️ Grouped by default | ✅ |
| Parallel jobs | ✅ `-j N` | ✅ `-P N` | ✅ `-j N` | ❌ Manual |
| Shell quoting | ✅ Auto-quoted `{}` | ❌ Manual | ✅ | N/A |
| Bash-style syntax | ✅ `rfor i in ... --` | ❌ | ❌ | ✅ Native |
| Named variables | ✅ `{url}`, `{host}` | ❌ | ❌ | ✅ `$var` |
| Install | Single binary | Built-in | Package manager | Built-in |
| Halt on fail | ✅ `--halt-on-fail` | ❌ | ✅ `--halt` | `set -e` |

### When to use which syntax

| Use case | Recommended style | Why |
|----------|-------------------|-----|
| Quick one-liner | GNU parallel | `rfor 'echo {}' ::: a b c` — compact |
| Readable scripts | Bash for-loop | `rfor host in ... -- ssh {host} ...` — self-documenting |
| Piped input | Either | Both support stdin; GNU is shorter, bash names the var |
| Commands with literal braces | Bash for-loop | `{other}` passes through unless it matches the declared var |

## Caveats

- **`--` in items is not escapable.** If your items literally contain `--`,
  use an argfile or stdin instead of inline items:

  ```sh
  # Won't work — rfor sees -- as the separator:
  rfor i in --verbose --quiet -- echo {i}

  # Workaround — use a file or stdin:
  printf '%s\n' --verbose --quiet | rfor i -- echo {i}
  ```

## License

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) or
[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0), at your option.
