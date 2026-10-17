//! Harness entry point for the E2E `.feature` files: World and main() only.

mod steps;

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::gherkin::{Feature, Rule, Scenario};
use cucumber::{ScenarioType, World};

/// The local services are shared with the user's live session; more concurrent
/// clients make the speech recognition service reset connections it has no room for.
const MAX_CONCURRENT_SCENARIOS: usize = 4;
/// Tags of scenarios that run alone: the accuracy benchmarks load the services on purpose
/// (the corpus sample is transcribed two at a time), and a scenario beside them would wait
/// out its timeouts in the speech recognition service's queue.
const RUN_ALONE: [&str; 2] = ["serial", "accuracy"];

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
    pub local_services: Option<steps::local_services::ScenarioServices>,
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
    // One nextest test, `scenarios`, is the whole run; any other command line is cucumber's own
    let Some(command_line) = voicepuncher_e2e_test::harness_args::cucumber_command_line() else {
        return;
    };
    // The user's recorded commands stay on the machine; scenarios that need them run only
    // where they exist.
    let has_voice_samples =
        std::env::var("VOICEPUNCHER_VOICE_SAMPLES").is_ok_and(|d| !d.is_empty());
    if !has_voice_samples {
        eprintln!("GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping @voice_samples scenarios");
    }
    // The scenario probe for the suite limit (fail-hung-tests-fast, D10): armed, every scenario
    // blocks forever before its first step, so the suite's nextest limit must stop the harness.
    let hang_probe = std::env::var("VOICEPUNCHER_HANG_PROBE").as_deref() == Ok("1");
    futures::executor::block_on(
        E2eWorld::cucumber()
            .max_concurrent_scenarios(MAX_CONCURRENT_SCENARIOS)
            .which_scenario(|feature, rule, scenario| {
                if RUN_ALONE
                    .iter()
                    .any(|tag| has_tag(feature, rule, scenario, tag))
                {
                    ScenarioType::Serial
                } else {
                    ScenarioType::Concurrent
                }
            })
            .before(move |_feature, _rule, scenario, _world| {
                Box::pin(async move {
                    if hang_probe {
                        eprintln!(
                            "hang probe armed: holding scenario '{}' forever",
                            scenario.name
                        );
                        futures::future::pending::<()>().await;
                    }
                })
            })
            .fail_on_skipped()
            .with_cli(Opts::<_, _, _, Empty>::parse_from(command_line))
            .filter_run_and_exit("tests/features", move |feature, rule, scenario| {
                let tagged = |tag| has_tag(feature, rule, scenario, tag);
                !tagged("wip") && (has_voice_samples || !tagged("voice_samples"))
            }),
    );
}

fn has_tag(feature: &Feature, rule: Option<&Rule>, scenario: &Scenario, tag: &str) -> bool {
    let tagged = |tags: &[String]| tags.iter().any(|t| t == tag);
    tagged(&feature.tags) || rule.is_some_and(|r| tagged(&r.tags)) || tagged(&scenario.tags)
}
