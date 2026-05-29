# pfor

A parallel for-loop replacement with live output and a sticky progress bar.

`pfor` runs a shell command once per item — from inline arguments, a file, or
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
- **Familiar syntax** — GNU-parallel-style `:::` / `::::` separators and
  `{}` / `{#}` substitution tokens.

## Installation

### From source (requires [Rust ≥ 1.70](https://rustup.rs))

```sh
git clone https://github.com/<owner>/pfor.git
cd pfor
cargo install --path .
```

The binary lands in `~/.cargo/bin/pfor`.  Make sure that directory is on your
`PATH`.

### Build a release binary without installing

```sh
cargo build --release
# binary is at ./target/release/pfor
```

## Quick start

```sh
# Greet three items inline
pfor 'echo hello {}' ::: world rust pfor

# Download URLs listed in a file, 4 at a time
pfor -j 4 'curl -sO {}' :::: urls.txt

# Pipe items from another command
find . -name '*.log' | pfor 'gzip {}'
```

## Usage

```
pfor [OPTIONS] '<command template>' ::: item1 item2 ...
pfor [OPTIONS] '<command template>' :::: argfile
<stdin> | pfor [OPTIONS] '<command template>'
```

The first positional argument is always the **command template** — a shell
string executed via `sh -c` for each item.  Items come from one of three
sources (exactly one per invocation):

| Source | Syntax | Example |
|--------|--------|---------|
| Inline | `:::` followed by arguments | `pfor 'echo {}' ::: a b c` |
| File | `::::` followed by a file path | `pfor 'echo {}' :::: items.txt` |
| Stdin | pipe or redirect into `pfor` | `cat list \| pfor 'echo {}'` |

Blank lines in files and stdin are silently skipped.

> **Tip:** if you run `pfor` without `:::`, `::::`, or piped input, it reads
> from the terminal and prints a hint.  Press **Ctrl-D** to finish, or
> re-run with an explicit item source.

## Template tokens

Tokens inside the command template are expanded before each invocation:

| Token | Expands to | Example |
|-------|------------|---------|
| `{}`  | Current item (automatically shell-quoted) | `pfor 'echo {}'  ::: hello` → `echo 'hello'` |
| `{#}` | 1-based job index | `pfor 'echo {#}' ::: a b c` → `1`, `2`, `3` |
| `{{`  | Literal `{` | `pfor 'echo {{}}'` → `echo {}`  |
| `}}`  | Literal `}` | |

Items substituted via `{}` are POSIX-shell-quoted (single-quoted with
embedded `'` escaped), so filenames with spaces, quotes, or glob characters
are handled safely.

## Options

| Flag | Long | Default | Description |
|------|------|---------|-------------|
| `-j` | `--jobs N` | `1` | Number of parallel workers.  `0` = number of logical CPUs. |
| | `--halt-on-fail` | off | Stop scheduling new jobs after the first failure.  Jobs already running will finish. |
| `-h` | `--help` | | Print help and examples. |
| `-V` | `--version` | | Print version (`pfor 0.1.0`). |

### Concurrency (`-j`)

```sh
# Sequential (default) — one job at a time
pfor 'make -C {}' ::: proj_a proj_b proj_c

# 4 parallel workers
pfor -j 4 'convert {} {}.webp' :::: images.txt

# As many workers as CPU cores
pfor -j 0 'cargo test -p {}' ::: core api web
```

With `-j 1` (the default), jobs run in order and output appears exactly as it
would in a `for` loop.  With `-j N` where N > 1, up to N jobs run
concurrently; output lines from different jobs may interleave, but individual
lines are never torn.

### Halt on failure (`--halt-on-fail`)

```sh
pfor --halt-on-fail 'deploy {}' ::: staging production
```

Without `--halt-on-fail` (the default), all items run even if some fail —
matching the behavior of GNU `parallel`.  When `--halt-on-fail` is active,
no *new* jobs are started after a failure, but any in-flight jobs are allowed
to finish.

## Progress bar

On a TTY terminal, `pfor` displays a sticky progress bar on the bottom line:

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
`pfor` is safe to use in pipelines:

```sh
pfor 'process {}' :::: items.txt 2>/dev/null   # bar hidden, stdout clean
pfor 'process {}' :::: items.txt | tee out.log  # bar hidden automatically
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | All jobs succeeded (exit 0). |
| `1`–`125` | Number of failed jobs (capped at 125). |
| `2`  | Usage error (bad arguments, missing template, unreadable argfile). |

Examples:

```sh
pfor 'true' ::: a b c
echo $?   # 0

pfor 'false' ::: a b c
echo $?   # 3  (all three failed)

pfor 'sh -c "[ {} = b ] && exit 1 || true"' ::: a b c
echo $?   # 1  (one failure)
```

## Real-world examples

### Compress log files in parallel

```sh
find /var/log -name '*.log' -mtime +7 | pfor -j 4 'gzip -9 {}'
```

### Run tests across multiple packages

```sh
pfor -j 0 'cargo test -p {}' ::: core api web
```

### Batch image conversion

```sh
ls *.png | pfor -j 8 'convert {} -resize 800x600 resized/{}'
```

### Deploy to multiple hosts, stop on first failure

```sh
pfor --halt-on-fail 'ssh {} "sudo systemctl restart myapp"' \
    ::: host1 host2 host3
```

### Download URLs from a file

```sh
pfor -j 4 'curl -sfSL -o /tmp/{#}.html {}' :::: urls.txt
```

### Sequential build with progress tracking

```sh
pfor 'make -C {}' ::: lib1 lib2 lib3 app
```

## Comparison with other tools

| | `pfor` | `xargs -P` | GNU `parallel` | bash `for` loop |
|---|---|---|---|---|
| Progress bar | ✅ Sticky ETA bar | ❌ | ❌ (requires `--bar`) | ❌ |
| Live output | ✅ Streams above bar | ✅ (may interleave) | ⚠️ Grouped by default | ✅ |
| Parallel jobs | ✅ `-j N` | ✅ `-P N` | ✅ `-j N` | ❌ Manual |
| Shell quoting | ✅ Auto-quoted `{}` | ❌ Manual | ✅ | N/A |
| Install | Single binary | Built-in | Package manager | Built-in |
| Halt on fail | ✅ `--halt-on-fail` | ❌ | ✅ `--halt` | `set -e` |

## License

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) or
[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0), at your option.
