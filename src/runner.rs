//! Job orchestration: sequential and parallel runners.
//!
//! Architecture:
//! - A bounded job queue (crossbeam-channel) feeds `jobs` worker threads.
//! - Each worker spawns the child via `sh -c <rendered>`, pipes stdout/stderr,
//!   and spawns two short-lived reader threads that drain complete lines into
//!   the shared `Printer`.
//! - Failure count is shared via an atomic; `halt-on-fail` flips a stop flag
//!   that workers check before pulling the next job.

use crate::output::{Printer, Stream};
use crate::template;
use crossbeam_channel::{bounded, Receiver};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct RunConfig {
    pub template: String,
    pub items: Vec<String>,
    pub jobs: usize,
    pub halt_on_fail: bool,
    pub use_bar: bool,
    /// Named variable for bash-style syntax (e.g. `i`). `None` for GNU-parallel.
    pub var_name: Option<String>,
    /// Print commands without executing them.
    pub dry_run: bool,
    /// Buffer each job's output and print as a block on completion.
    pub group: bool,
    /// Number of times to retry a failed job before counting it as a failure.
    pub retries: usize,
    /// Directory to save each job's stdout/stderr output files.
    pub results_dir: Option<String>,
    /// Show per-worker slot bars alongside the main progress bar.
    pub multi_bar: bool,
}

/// Result summary returned to main for exit-code computation.
#[allow(dead_code)]
pub struct RunSummary {
    pub total: usize,
    pub failures: usize,
}

/// Job dispatched to a worker.
struct Job {
    index: usize, // 1-based
    item: String,
}

/// Shared context passed to each worker thread.
struct WorkerCtx {
    printer: Arc<Printer>,
    failures: Arc<AtomicUsize>,
    halt: Arc<AtomicBool>,
    template: String,
    halt_on_fail: bool,
    bar: Option<ProgressBar>,
    var_name: Option<String>,
    dry_run: bool,
    group: bool,
    retries: usize,
    results_dir: Option<String>,
    /// Per-worker slot bar for multi-bar mode. `None` when not enabled.
    slot_bar: Option<ProgressBar>,
}

/// Run all jobs according to `cfg`. Returns the summary.
pub fn run(cfg: RunConfig) -> RunSummary {
    let total = cfg.items.len();
    if total == 0 {
        // Nothing to do. Still return a clean summary.
        return RunSummary { total: 0, failures: 0 };
    }

    // Choose mode + build printer/bar.
    let use_multi_bar = cfg.multi_bar && cfg.use_bar && cfg.jobs > 1;
    let (printer, bar_opt, slot_bars): (Arc<Printer>, Option<ProgressBar>, Vec<Option<ProgressBar>>) = if cfg.use_bar {
        let mp = MultiProgress::new();

        // Create per-slot bars ABOVE the main bar when multi-bar is enabled.
        let slots: Vec<Option<ProgressBar>> = if use_multi_bar {
            let slot_style = ProgressStyle::with_template("  {prefix:.dim} {msg}")
                .expect("valid slot template");
            (0..cfg.jobs)
                .map(|i| {
                    let sb = mp.add(ProgressBar::new_spinner());
                    sb.set_style(slot_style.clone());
                    sb.set_prefix(format!("[slot {}]", i + 1));
                    sb.set_message("idle");
                    sb.enable_steady_tick(Duration::from_millis(200));
                    Some(sb)
                })
                .collect()
        } else {
            (0..cfg.jobs).map(|_| None).collect()
        };

        let bar = mp.add(ProgressBar::new(total as u64));
        bar.set_style(
            ProgressStyle::with_template(
                "{bar:40.cyan/blue} {pos}/{len} [{elapsed_precise}] eta {eta_precise}",
            )
            .expect("valid template")
            .progress_chars("=>-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        (Arc::new(Printer::bar(mp)), Some(bar), slots)
    } else {
        let slots: Vec<Option<ProgressBar>> = (0..cfg.jobs).map(|_| None).collect();
        (Arc::new(Printer::plain()), None, slots)
    };

    let failures = Arc::new(AtomicUsize::new(0));
    let halt = Arc::new(AtomicBool::new(false));

    // Bounded channel: cap at jobs*2 so the producer doesn't race far ahead
    // (matters when halt-on-fail trips — fewer queued jobs to discard).
    let cap = cfg.jobs.saturating_mul(2).max(1);
    let (tx, rx) = bounded::<Job>(cap);

    // Producer thread: feeds jobs, respects halt flag.
    let producer = {
        let halt = Arc::clone(&halt);
        let items = cfg.items.clone();
        thread::spawn(move || {
            for (i, item) in items.into_iter().enumerate() {
                if halt.load(Ordering::Acquire) {
                    break;
                }
                let job = Job { index: i + 1, item };
                if tx.send(job).is_err() {
                    break;
                }
            }
            // Dropping tx signals workers to exit once the queue drains.
        })
    };

    // Worker threads.
    let mut workers = Vec::with_capacity(cfg.jobs);
    for (worker_idx, slot_bar) in slot_bars.into_iter().enumerate() {
        let _ = worker_idx;
        let rx = rx.clone();
        let ctx = WorkerCtx {
            printer: Arc::clone(&printer),
            failures: Arc::clone(&failures),
            halt: Arc::clone(&halt),
            template: cfg.template.clone(),
            halt_on_fail: cfg.halt_on_fail,
            bar: bar_opt.clone(),
            var_name: cfg.var_name.clone(),
            dry_run: cfg.dry_run,
            group: cfg.group,
            retries: cfg.retries,
            results_dir: cfg.results_dir.clone(),
            slot_bar,
        };
        workers.push(thread::spawn(move || {
            worker_loop(rx, &ctx);
        }));
    }
    drop(rx);

    let _ = producer.join();
    for w in workers {
        let _ = w.join();
    }

    if let Some(b) = bar_opt {
        b.finish_and_clear();
    }

    // Note: slot bars are owned by workers and cleared when worker_loop exits.

    RunSummary {
        total,
        failures: failures.load(Ordering::Acquire),
    }
}

fn worker_loop(rx: Receiver<Job>, ctx: &WorkerCtx) {
    while let Ok(job) = rx.recv() {
        if ctx.halt.load(Ordering::Acquire) {
            break;
        }
        // Update slot bar with current item.
        if let Some(ref sb) = ctx.slot_bar {
            sb.set_message(format!("running: {}", job.item));
        }
        let rendered = template::render(&ctx.template, &job.item, job.index, ctx.var_name.as_deref());
        if ctx.dry_run {
            ctx.printer.println(Stream::Stdout, &rendered);
        } else {
            let max_attempts = ctx.retries + 1;
            let mut ok = false;
            // When --results or --group is active, we need collected output.
            let need_collect = ctx.group || ctx.results_dir.is_some();
            let mut final_result: Option<CollectedOutput> = None;

            for attempt in 1..=max_attempts {
                if attempt > 1 {
                    ctx.printer.println(
                        Stream::Stderr,
                        &format!(
                            "rfor: retrying job {index} (attempt {attempt}/{max_attempts})...",
                            index = job.index,
                        ),
                    );
                }
                if need_collect {
                    let result = spawn_and_collect(&rendered);
                    ok = result.success;
                    if ok || attempt == max_attempts {
                        // Emit output (grouped or not).
                        if ctx.group {
                            let mut block: Vec<(Stream, &str)> = Vec::new();
                            for line in &result.stdout_lines {
                                block.push((Stream::Stdout, line));
                            }
                            for line in &result.stderr_lines {
                                block.push((Stream::Stderr, line));
                            }
                            ctx.printer.println_block(&block);
                        } else {
                            // --results without --group: stream the collected lines.
                            for line in &result.stdout_lines {
                                ctx.printer.println(Stream::Stdout, line);
                            }
                            for line in &result.stderr_lines {
                                ctx.printer.println(Stream::Stderr, line);
                            }
                        }
                        final_result = Some(result);
                    }
                } else {
                    ok = spawn_and_stream(&rendered, &ctx.printer);
                }
                if ok {
                    break;
                }
            }

            // Write results files if --results is set.
            if let (Some(ref dir), Some(ref result)) = (&ctx.results_dir, &final_result) {
                write_results(dir, job.index, &job.item, result);
            }

            if !ok {
                ctx.failures.fetch_add(1, Ordering::AcqRel);
                if ctx.halt_on_fail {
                    ctx.halt.store(true, Ordering::Release);
                }
            }
        }
        if let Some(b) = &ctx.bar {
            b.inc(1);
        }
        // Clear slot bar after job completes.
        if let Some(ref sb) = ctx.slot_bar {
            sb.set_message("idle");
        }
    }
    // Worker exiting — clear the slot bar.
    if let Some(ref sb) = ctx.slot_bar {
        sb.finish_and_clear();
    }
}

/// Sanitize an item string for use as a filename component.
/// Replaces `/` with `_` and truncates to 200 chars.
fn sanitize_item(item: &str) -> String {
    let s: String = item
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Collapse runs of underscores and truncate.
    let collapsed: String = s.chars().fold(String::new(), |mut acc, c| {
        if c == '_' && acc.ends_with('_') {
            acc
        } else {
            acc.push(c);
            acc
        }
    });
    if collapsed.len() > 200 {
        collapsed[..200].to_string()
    } else {
        collapsed
    }
}

/// Write stdout and stderr of a completed job to result files.
fn write_results(dir: &str, index: usize, item: &str, output: &CollectedOutput) {
    let safe = sanitize_item(item);
    let base = format!("{}-{}", index, safe);

    let out_path = Path::new(dir).join(format!("{}.out", base));
    if let Ok(mut f) = fs::File::create(&out_path) {
        for line in &output.stdout_lines {
            let _ = writeln!(f, "{}", line);
        }
    }

    let err_path = Path::new(dir).join(format!("{}.err", base));
    if let Ok(mut f) = fs::File::create(&err_path) {
        for line in &output.stderr_lines {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// Output collected from a grouped (buffered) job execution.
struct CollectedOutput {
    success: bool,
    stdout_lines: Vec<String>,
    stderr_lines: Vec<String>,
}

/// Spawn `sh -c rendered`, collect all stdout/stderr lines, return them with
/// the exit status. Used when `--group` is active.
fn spawn_and_collect(rendered: &str) -> CollectedOutput {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(rendered)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return CollectedOutput {
                success: false,
                stdout_lines: Vec::new(),
                stderr_lines: vec![format!("rfor: failed to spawn `sh -c`: {}", e)],
            };
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let t_out = stdout.map(|s| {
        thread::spawn(move || {
            BufReader::new(s)
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
        })
    });
    let t_err = stderr.map(|s| {
        thread::spawn(move || {
            BufReader::new(s)
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
        })
    });

    let status = child.wait();
    let stdout_lines = t_out.and_then(|t| t.join().ok()).unwrap_or_default();
    let stderr_lines = t_err.and_then(|t| t.join().ok()).unwrap_or_default();

    CollectedOutput {
        success: matches!(status, Ok(s) if s.success()),
        stdout_lines,
        stderr_lines,
    }
}

/// Spawn `sh -c rendered`, stream stdout/stderr through `printer`, return true
/// if the child exited 0.
fn spawn_and_stream(rendered: &str, printer: &Arc<Printer>) -> bool {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(rendered)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            printer.println(
                Stream::Stderr,
                &format!("rfor: failed to spawn `sh -c`: {}", e),
            );
            return false;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let p1 = Arc::clone(printer);
    let t_out = stdout.map(|s| {
        thread::spawn(move || {
            for line in BufReader::new(s).lines().map_while(Result::ok) {
                p1.println(Stream::Stdout, &line);
            }
        })
    });
    let p2 = Arc::clone(printer);
    let t_err = stderr.map(|s| {
        thread::spawn(move || {
            for line in BufReader::new(s).lines().map_while(Result::ok) {
                p2.println(Stream::Stderr, &line);
            }
        })
    });

    let status = child.wait();
    if let Some(t) = t_out {
        let _ = t.join();
    }
    if let Some(t) = t_err {
        let _ = t.join();
    }

    matches!(status, Ok(s) if s.success())
}
