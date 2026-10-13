//! Harness entry point for the E2E `.feature` files: World and main() only.

mod steps;

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

use cucumber::World;

/// The local services are shared with the user's live session; more concurrent
/// clients make the speech recognition service reset connections it has no room for.
const MAX_CONCURRENT_SCENARIOS: usize = 4;

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
    pub refinement_report: Option<voicepuncher_e2e_test::refinement_eval::Report>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Option<Duration>,
    pub voice_control: bool,
    pub command_spotting_url: Option<String>,
    pub recognized: Vec<voicepuncher::core::domain::VoiceCommand>,
    pub listened_before_secs: f32,
    pub hands_free: Option<steps::voice_control::HandsFree>,
    pub session: Option<steps::voice_control::HandsFreeSession>,
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
    // The user's recorded commands stay on the machine; scenarios that need them run only
    // where they exist.
    let has_voice_samples =
        std::env::var("VOICEPUNCHER_VOICE_SAMPLES").is_ok_and(|d| !d.is_empty());
    if !has_voice_samples {
        eprintln!("GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping @voice_samples scenarios");
    }
    futures::executor::block_on(
        E2eWorld::cucumber()
            .max_concurrent_scenarios(MAX_CONCURRENT_SCENARIOS)
            .fail_on_skipped()
            .filter_run_and_exit("tests/features", move |feature, rule, scenario| {
                let tagged = |tag: &str| {
                    let has = |tags: &[String]| tags.iter().any(|t| t == tag);
                    has(&feature.tags) || rule.is_some_and(|r| has(&r.tags)) || has(&scenario.tags)
                };
                !tagged("wip") && (has_voice_samples || !tagged("voice_samples"))
            }),
    );
}
