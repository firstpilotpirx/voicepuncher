//! Harness entry point for the E2E `.feature` files: World and main() only.

mod steps;

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

use cucumber::World;

#[derive(Debug, Default, World)]
pub struct E2eWorld {
    pub workdir: Option<tempfile::TempDir>,
    pub recording: Option<PathBuf>,
    pub spoken: Option<voicepuncher_e2e_test::speech::Lang>,
    pub exit_status: Option<ExitStatus>,
    pub corpus: Vec<voicepuncher_e2e_test::corpus::Sample>,
    pub word_error_rates: Vec<f64>,
    pub config: Option<PathBuf>,
    pub transcription_url: Option<String>,
    pub refinement_url: Option<String>,
    pub refinement_stub: Option<steps::refinement_service_stub::RefinementServiceStub>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Option<Duration>,
}

impl E2eWorld {
    /// Temporary directory owned by the scenario, created on first use.
    pub fn workdir(&mut self) -> anyhow::Result<&Path> {
        if self.workdir.is_none() {
            self.workdir = Some(tempfile::tempdir()?);
        }
        Ok(self
            .workdir
            .as_ref()
            .expect("workdir was just created")
            .path())
    }
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
