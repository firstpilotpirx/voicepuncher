//! Harness entry point for the slice and component `.feature` files: World and main() only.

mod steps;

use std::sync::mpsc::Receiver;

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::gherkin::{Feature, Rule, Scenario};
use cucumber::World;

use voicepuncher::core::domain::PttSignal;

#[derive(Debug, Default, World)]
pub struct IntegrationWorld {
    pub ptt_signals: Option<Receiver<PttSignal>>,
    pub dictation_language: Option<String>,
    pub dictation: Option<steps::speech_recognition::Dictation>,
    pub pasted: Option<String>,
    pub insertion: Option<steps::text_insertion::InsertionRun>,
    pub voice: Option<steps::voice_control::VoiceRun>,
    pub stages: Option<steps::processing_stages::StagesRun>,
}

fn main() {
    // One nextest test, `scenarios`, is the whole run; any other command line is cucumber's own
    let Some(command_line) = voicepuncher_e2e_test::harness_args::cucumber_command_line() else {
        return;
    };
    let real_keyboard = std::env::var("VOICEPUNCHER_REAL_KEYBOARD_IT").is_ok();
    if !real_keyboard {
        eprintln!(
            "GATED: VOICEPUNCHER_REAL_KEYBOARD_IT not set, skipping @real_keyboard scenarios"
        );
    }
    futures::executor::block_on(
        IntegrationWorld::cucumber()
            .fail_on_skipped()
            .with_cli(Opts::<_, _, _, Empty>::parse_from(command_line))
            .filter_run_and_exit("tests/features", move |feature, rule, scenario| {
                let excluded = has_tag(feature, rule, scenario, "wip")
                    || (!real_keyboard && has_tag(feature, rule, scenario, "real_keyboard"));
                !excluded
            }),
    );
}

fn has_tag(feature: &Feature, rule: Option<&Rule>, scenario: &Scenario, tag: &str) -> bool {
    let tagged = |tags: &[String]| tags.iter().any(|t| t == tag);
    tagged(&feature.tags) || rule.is_some_and(|r| tagged(&r.tags)) || tagged(&scenario.tags)
}
