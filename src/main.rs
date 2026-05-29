//! pfor entry point.

mod cli;
mod expand;
mod output;
mod runner;
mod source;
mod template;
mod tty;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();

    let args = match cli::split_rest(parsed.rest) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("pfor: {}", e);
            return ExitCode::from(2);
        }
    };

    let items = match source::resolve(args.inline_items, args.argfile) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("pfor: failed to read items: {}", e);
            return ExitCode::from(2);
        }
    };

    // Expand brace expressions in items ({1..10}, {a..z}, etc.).
    let items = match expand::expand_items(items) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("pfor: {}", e);
            return ExitCode::from(2);
        }
    };

    // Create results directory if --results is specified.
    if let Some(ref dir) = parsed.results {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("pfor: failed to create results directory `{}`: {}", dir, e);
            return ExitCode::from(2);
        }
    }

    let jobs = if parsed.jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        parsed.jobs
    };

    let use_bar = tty::stderr_is_tty();

    // Warn once if the template uses GNU parallel tokens pfor doesn't support.
    template::warn_unsupported_tokens(&args.template, args.var_name.as_deref());

    let summary = runner::run(runner::RunConfig {
        template: args.template,
        items,
        jobs,
        halt_on_fail: parsed.halt_on_fail,
        use_bar,
        var_name: args.var_name,
        dry_run: parsed.dry_run,
        group: parsed.group,
        retries: parsed.retries,
        results_dir: parsed.results,
        multi_bar: parsed.multi_bar,
    });

    if summary.failures == 0 {
        ExitCode::SUCCESS
    } else {
        let code = std::cmp::min(summary.failures, 125) as u8;
        ExitCode::from(code)
    }
}
