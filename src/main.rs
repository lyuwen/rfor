//! pfor entry point.

mod cli;
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
    });

    if summary.failures == 0 {
        ExitCode::SUCCESS
    } else {
        let code = std::cmp::min(summary.failures, 125) as u8;
        ExitCode::from(code)
    }
}
