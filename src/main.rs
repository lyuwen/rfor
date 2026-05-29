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

    let (template_str, inline_items, argfile) = match cli::split_rest(parsed.rest) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("pfor: {}", e);
            return ExitCode::from(2);
        }
    };

    let items = match source::resolve(inline_items, argfile) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("pfor: failed to read items: {}", e);
            return ExitCode::from(2);
        }
    };

    let jobs = if parsed.jobs == 0 {
        num_cpus::get()
    } else {
        parsed.jobs
    };

    let use_bar = tty::stderr_is_tty();

    let summary = runner::run(runner::RunConfig {
        template: template_str,
        items,
        jobs,
        halt_on_fail: parsed.halt_on_fail,
        use_bar,
    });

    if summary.failures == 0 {
        ExitCode::SUCCESS
    } else {
        let code = std::cmp::min(summary.failures, 125) as u8;
        ExitCode::from(code)
    }
}
