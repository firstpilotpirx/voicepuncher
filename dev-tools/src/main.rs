//! `dev-tools <command> [args]`: what the `justfile`'s recipes call (AGENTS.md § How to verify a change) —
//! the stage runner (keys, pass records, lock phases, the stage table and journal), the frozen tests, the named
//! runs, the filters of the test selections, the engines and the local services.

mod accuracy;
mod accuracy_baseline;
mod clock;
mod context;
mod engines;
mod filters;
mod frozen;
mod journal;
#[cfg(test)]
mod justfile_rules;
mod keys;
mod locks;
mod named;
mod passes;
mod process;
mod services;
mod stages;

use std::process::ExitCode;

/// What a command returns: its exit code, or a message printed after `✗ ` with exit code 1.
pub type Outcome = Result<u8, String>;

type Command = fn(&[String]) -> Outcome;

/// Every command, by the name a recipe calls it with.
const COMMANDS: [(&str, Command); 24] = [
    ("stages", stages::command),
    ("model-stages", |stages| Ok(stages::model::command(stages))),
    ("test-stage", stages::test::command),
    ("it-real", stages::it_real::command),
    ("named-run", named::command),
    ("rerun-failed", named::rerun::command),
    ("engines", engines::command),
    ("check-bundle", engines::bundle::command),
    ("locked", locks::locked_command),
    ("require-lock", locks::require_command),
    ("accuracy-stage", accuracy::command),
    ("accuracy-baseline", accuracy_baseline::command),
    ("filter", filters::command),
    ("gated-filter", filters::gated_command),
    ("keys", keys::command),
    ("stage-times", journal::times::command),
    ("frozen", frozen::command),
    ("frozen-tests", frozen::run::command),
    ("freeze-test", frozen::freeze::command),
    ("start-model-servers", services::start_command),
    ("stop-model-servers", services::stop_command),
    ("model-servers-for-e2e", services::e2e_command),
    ("supervise", services::supervise_command),
    ("help", |_| {
        usage();
        Ok(0)
    }),
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((name, rest)) = args.split_first() else {
        usage();
        return ExitCode::from(2);
    };
    let Some((_, command)) = COMMANDS.iter().find(|(known, _)| known == name) else {
        eprintln!("✗ unknown command '{name}'");
        usage();
        return ExitCode::from(2);
    };
    match command(rest) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("✗ {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    let names: Vec<&str> = COMMANDS.iter().map(|(name, _)| *name).collect();
    eprintln!("usage: dev-tools <command> [args] — {}", names.join(", "));
}
