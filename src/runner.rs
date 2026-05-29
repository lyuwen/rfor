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
use std::io::{BufRead, BufReader};
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
}

/// Run all jobs according to `cfg`. Returns the summary.
pub fn run(cfg: RunConfig) -> RunSummary {
    let total = cfg.items.len();
    if total == 0 {
        // Nothing to do. Still return a clean summary.
        return RunSummary { total: 0, failures: 0 };
    }

    // Choose mode + build printer/bar.
    let (printer, bar_opt): (Arc<Printer>, Option<ProgressBar>) = if cfg.use_bar {
        let mp = MultiProgress::new();
        let bar = mp.add(ProgressBar::new(total as u64));
        bar.set_style(
            ProgressStyle::with_template(
                "{bar:40.cyan/blue} {pos}/{len} [{elapsed_precise}] eta {eta_precise}",
            )
            .expect("valid template")
            .progress_chars("=>-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        (Arc::new(Printer::bar(mp)), Some(bar))
    } else {
        (Arc::new(Printer::plain()), None)
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
    for _ in 0..cfg.jobs {
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
        let rendered = template::render(&ctx.template, &job.item, job.index, ctx.var_name.as_deref());
        if ctx.dry_run {
            ctx.printer.println(Stream::Stdout, &rendered);
        } else {
            let max_attempts = ctx.retries + 1;
            let mut ok = false;
            for attempt in 1..=max_attempts {
                if attempt > 1 {
                    ctx.printer.println(
                        Stream::Stderr,
                        &format!(
                            "pfor: retrying job {index} (attempt {attempt}/{max_attempts})...",
                            index = job.index,
                        ),
                    );
                }
                if ctx.group {
                    let result = spawn_and_collect(&rendered);
                    ok = result.success;
                    // If grouped + retries, only emit the final attempt's output.
                    if ok || attempt == max_attempts {
                        let mut block: Vec<(Stream, &str)> = Vec::new();
                        for line in &result.stdout_lines {
                            block.push((Stream::Stdout, line));
                        }
                        for line in &result.stderr_lines {
                            block.push((Stream::Stderr, line));
                        }
                        ctx.printer.println_block(&block);
                    }
                } else {
                    ok = spawn_and_stream(&rendered, &ctx.printer);
                }
                if ok {
                    break;
                }
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
                stderr_lines: vec![format!("pfor: failed to spawn `sh -c`: {}", e)],
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
                &format!("pfor: failed to spawn `sh -c`: {}", e),
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
