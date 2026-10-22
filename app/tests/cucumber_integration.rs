//! Harness entry point for the slice and component `.feature` files: World and `main()` only.

#[allow(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber fixes step signatures: the world by &mut, captures by value"
)]
mod steps;
mod support;

use std::path::Path;
use std::sync::mpsc::Receiver;

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::World;
use voicepuncher_e2e_test::harness_args;
use voicepuncher_e2e_test::scenario_filter::{self, Gate};

use voicepuncher::core::domain::PttSignal;

/// The folder of the slice and component feature files, relative to the crate.
const FEATURES: &str = "tests/features";

#[derive(Debug, Default, World)]
pub struct IntegrationWorld {
    pub ptt_signals: Option<Receiver<PttSignal>>,
    pub dictation_language: Option<String>,
    pub dictation: Option<steps::speech_recognition::Dictation>,
    pub pasted: Option<String>,
    pub loss: Option<steps::speech_recognition::losses::LossRun>,
    pub insertion: Option<steps::text_insertion::InsertionRun>,
    pub voice: Option<steps::voice_control::VoiceRun>,
    pub stages: Option<steps::processing_stages::StagesRun>,
    pub live: Option<steps::live_transcript::LiveRun>,
    pub notices: Option<steps::failure_notices::NoticesRun>,
    pub pointer: Option<steps::dictation_pointer::PointerRun>,
    pub archive: Option<steps::dictation_archive::ArchiveRun>,
    pub runtime: Option<steps::app_runtime::RuntimeRun>,
    pub menu_bar: Option<steps::menu_bar_presence::MenuBarRun>,
    pub logging: Option<steps::app_logging::LoggingRun>,
    pub settings: Option<steps::settings_window::SettingsRun>,
    pub login: Option<steps::launch_at_login::LoginRun>,
    pub models: Option<steps::model_download::DownloadRun>,
}

/// Runs the scenarios the command line selects (a rerun's names instead, when
/// `VOICEPUNCHER_RERUN_SCENARIOS` holds any) that the gates admit: `@wip` never,
/// `@real_keyboard` only with `VOICEPUNCHER_REAL_KEYBOARD_IT` set.
///
/// cucumber never gets the selection (`--name`, `--tags`, `CUCUMBER_FILTER_TAGS`): it would apply
/// it instead of the filter, and the gates with it. `scenario_filter::split` takes it out of the
/// options and into the one filter that requires both.
fn main() {
    // One nextest test per rule, so nextest runs the rules in parallel processes and no long
    // feature file bounds the run; any other command line is cucumber's own and runs every file
    let tests = harness_args::harness_tests(Path::new(FEATURES));
    let Some((test, command_line)) = harness_args::rule_command_line(&tests) else {
        return;
    };
    let path = match &test {
        Some(test) => format!("{FEATURES}/{}.feature", test.feature),
        None => FEATURES.to_string(),
    };
    let gates = vec![
        Gate::wip(),
        Gate::real_keyboard(std::env::var("VOICEPUNCHER_REAL_KEYBOARD_IT")),
    ];
    let (opts, filter) =
        scenario_filter::split(Opts::<_, _, _, Empty>::parse_from(command_line), gates);
    let filter = filter.rerun_only(scenario_filter::rerun_names(std::env::var(
        scenario_filter::RERUN_VARIABLE,
    )));
    for line in filter.gated_lines() {
        eprintln!("{line}");
    }
    futures::executor::block_on(
        IntegrationWorld::cucumber()
            .fail_on_skipped()
            .with_cli(opts)
            .filter_run_and_exit(path, move |feature, rule, scenario| {
                test.as_ref()
                    .is_none_or(|test| test.takes(rule.map(|r| r.name.as_str())))
                    && filter.runs(feature, rule, scenario)
            }),
    );
}
