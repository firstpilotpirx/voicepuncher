//! Harness entry point for the E2E `.feature` files: World and main() only.

mod steps;

use std::path::PathBuf;
use std::process::ExitStatus;

use cucumber::World;

#[derive(Debug, Default, World)]
pub struct E2eWorld {
    pub workdir: Option<tempfile::TempDir>,
    pub recording: Option<PathBuf>,
    pub exit_status: Option<ExitStatus>,
    pub corpus: Vec<voicepuncher_e2e_test::corpus::Sample>,
    pub word_error_rates: Vec<f64>,
}

fn main() {
    futures::executor::block_on(E2eWorld::cucumber().fail_on_skipped().filter_run_and_exit(
        "tests/features",
        |feature, rule, scenario| {
            let wip = |tags: &[String]| tags.iter().any(|t| t == "wip");
            !wip(&feature.tags) && !rule.is_some_and(|r| wip(&r.tags)) && !wip(&scenario.tags)
        },
    ));
}
