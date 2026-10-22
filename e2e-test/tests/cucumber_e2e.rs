//! Harness entry point for the E2E `.feature` files: World and `main()` only.

#[allow(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber fixes step signatures: the world by &mut, captures by value"
)]
mod steps;

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::{ScenarioType, World};
use voicepuncher_e2e_test::harness_args;
use voicepuncher_e2e_test::scenario_filter::{self, has_tag, Gate};

/// Tags of scenarios that run alone: the accuracy benchmarks load the services on purpose
/// (the corpus sample is transcribed two at a time), and a scenario beside them would wait
/// out its timeouts in the speech recognition service's queue.
const RUN_ALONE: [&str; 2] = ["serial", "accuracy"];

#[derive(Debug, Default, World)]
pub struct E2eWorld {
    pub workdir: Option<tempfile::TempDir>,
    pub recording: Option<PathBuf>,
    pub spoken: Option<voicepuncher_e2e_test::speech::Lang>,
    /// A transcript the user dictates without a recording, refined as the app refines it.
    pub dictated: Option<&'static str>,
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
    /// The log file of a hotkey-mode run, which logs nothing to a piped error output
    pub app_log: String,
    pub elapsed: Option<Duration>,
    pub voice_control: bool,
    /// The app under test starts its services itself; off leaves them to the service scripts.
    pub manage_services: bool,
    pub command_spotting_url: Option<String>,
    pub recognized: Vec<voicepuncher::core::domain::VoiceCommand>,
    pub listened_before_secs: f32,
    pub hands_free: Option<steps::voice_control::HandsFree>,
    pub session: Option<steps::voice_control::HandsFreeSession>,
    pub local_services: Option<steps::local_services::ScenarioServices>,
    /// Services the app's own supervisor started for the scenario, and their outages.
    pub owned_services: steps::local_services::OwnedServices,
    pub archive: steps::dictation_archive::ArchiveRun,
    pub distribution: steps::app_distribution::DistributionRun,
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

/// Runs the scenarios the command line selects (a rerun's names instead, when
/// `VOICEPUNCHER_RERUN_SCENARIOS` holds any) that the gates admit: `@wip` never, `@voice_samples`
/// only where the user's recordings exist, `@signing_identity` only where the certificate is named.
///
/// cucumber never gets the selection (`--name`, `--tags`, `CUCUMBER_FILTER_TAGS`): it would apply
/// it instead of the filter, and the gates with it. `scenario_filter::split` takes it out of the
/// options and into the one filter that requires both.
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn main() {
    // One nextest test, `scenarios`, is the whole run; any other command line is cucumber's own
    let Some(command_line) = harness_args::cucumber_command_line() else {
        return;
    };
    // check and full-check stop at the first failed scenario
    let command_line =
        harness_args::with_fail_fast(command_line, std::env::var(harness_args::FAIL_FAST_VAR));
    // The app's log lines, such as its `voice start:` timing, only for the modules RUST_LOG names;
    // the scenarios read the warnings and notes the app logs whatever RUST_LOG says
    steps::local_services::capture_log(
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).build(),
    );
    // The user's recorded commands stay on the machine; scenarios that need them run only
    // where they exist.
    let gates = vec![
        Gate::wip(),
        Gate::voice_samples(std::env::var("VOICEPUNCHER_VOICE_SAMPLES")),
        Gate::signing_identity(std::env::var("VOICEPUNCHER_SIGNING_IDENTITY")),
    ];
    let (opts, filter) =
        scenario_filter::split(Opts::<_, _, _, Empty>::parse_from(command_line), gates);
    let filter = filter.rerun_only(scenario_filter::rerun_names(std::env::var(
        scenario_filter::RERUN_VARIABLE,
    )));
    for line in filter.gated_lines() {
        eprintln!("{line}");
    }
    // The local services are shared with the user's live session; more concurrent clients make
    // the speech recognition service reset connections it has no room for
    let concurrent_scenarios =
        harness_args::concurrent_scenarios(std::env::var(harness_args::CONCURRENCY_VAR))
            .unwrap_or_else(|e| panic!("{e}"));
    // The scenario probe for the suite limit (fail-hung-tests-fast, D10): armed, every scenario
    // blocks forever before its first step, so the suite's nextest limit must stop the harness.
    let hang_probe = std::env::var("VOICEPUNCHER_HANG_PROBE").as_deref() == Ok("1");
    futures::executor::block_on(
        E2eWorld::cucumber()
            .max_concurrent_scenarios(concurrent_scenarios)
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
            .with_cli(opts)
            .filter_run_and_exit("tests/features", move |feature, rule, scenario| {
                filter.runs(feature, rule, scenario)
            }),
    );
}
