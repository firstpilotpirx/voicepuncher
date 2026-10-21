//! Harness entry point for the slice and component `.feature` files: World and main() only.

mod steps;

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
    pub insertion: Option<steps::text_insertion::InsertionRun>,
    pub voice: Option<steps::voice_control::VoiceRun>,
    pub stages: Option<steps::processing_stages::StagesRun>,
    pub live: Option<steps::live_transcript::LiveRun>,
    pub notices: Option<steps::failure_notices::NoticesRun>,
    pub pointer: Option<steps::dictation_pointer::PointerRun>,
    pub archive: Option<steps::dictation_archive::ArchiveRun>,
    pub logging: Option<steps::app_logging::LoggingRun>,
}

/// Runs the scenarios the command line selects that the gates admit: `@wip` never,
/// `@real_keyboard` only with `VOICEPUNCHER_REAL_KEYBOARD_IT` set.
///
/// cucumber never gets the selection (`--name`, `--tags`, `CUCUMBER_FILTER_TAGS`): it would apply
/// it instead of the filter, and the gates with it. `scenario_filter::split` takes it out of the
/// options and into the one filter that requires both.
fn main() {
    // One nextest test per feature file, so nextest runs the files in parallel processes; any
    // other command line is cucumber's own and runs every file
    let features = harness_args::feature_names(Path::new(FEATURES));
    let Some((feature, command_line)) = harness_args::feature_command_line(&features) else {
        return;
    };
    let path = match feature {
        Some(name) => format!("{FEATURES}/{name}.feature"),
        None => FEATURES.to_string(),
    };
    let gates = vec![
        Gate::wip(),
        Gate::real_keyboard(std::env::var("VOICEPUNCHER_REAL_KEYBOARD_IT")),
    ];
    let (opts, filter) =
        scenario_filter::split(Opts::<_, _, _, Empty>::parse_from(command_line), gates);
    for line in filter.gated_lines() {
        eprintln!("{line}");
    }
    futures::executor::block_on(
        IntegrationWorld::cucumber()
            .fail_on_skipped()
            .with_cli(opts)
            .filter_run_and_exit(path, move |feature, rule, scenario| {
                filter.runs(feature, rule, scenario)
            }),
    );
}
