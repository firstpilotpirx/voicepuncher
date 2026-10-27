//! Given steps: what the archive holds and whether the window is open.

use anyhow::Result;
use cucumber::given;

use voicepuncher::core::application::DictationOutcome;

use super::archive::Said;
use super::run;
use crate::IntegrationWorld;

pub(super) const TEXTS: [&str; 3] = ["first dictation", "second dictation", "third dictation"];
pub(super) const TO_PASTE: &str = "Text that was to be pasted.";
pub(super) const ONLY_RAW: &str = "only the raw transcript";
/// The second the damaged entry's dictation started at.
pub(super) const DAMAGED_SECOND: u64 = 2;
pub(super) const MANY: u64 = 305;

#[given("the archive holds three dictations with text, made one after the other")]
fn three_dictations(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    for (second, text) in (1..).zip(TEXTS) {
        run.keep(second, &Said::pasted(text));
    }
    Ok(())
}

#[given("the archive holds a dictation whose paste failed and one with only its raw transcript")]
fn paste_failed_and_raw(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    let raw_only = Said { pasted: None, raw: Some(ONLY_RAW), outcome: DictationOutcome::NoText };
    run.keep(1, &raw_only);
    let failed = Said {
        pasted: Some(TO_PASTE),
        raw: Some("text to paste"),
        outcome: DictationOutcome::PasteFailed,
    };
    run.keep(2, &failed);
    Ok(())
}

#[given("the archive holds a dictation on which speech recognition failed and one with text")]
fn failed_and_text(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    run.keep(1, &Said::pasted(TEXTS[0]));
    let failed =
        Said { pasted: None, raw: None, outcome: DictationOutcome::SpeechRecognitionFailed };
    run.keep(2, &failed);
    Ok(())
}

#[given("the archive holds 305 dictations with text")]
fn many_dictations(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    for second in 1..=MANY {
        run.keep(second, &Said::pasted(&format!("dictation {second}")));
    }
    Ok(())
}

#[given("the history window lists two dictations")]
fn window_lists_two(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    run.keep(1, &Said::pasted(TEXTS[0]));
    run.keep(2, &Said::pasted(TEXTS[1]));
    run.open();
    Ok(())
}

#[given("the history window is open and lists one dictation")]
fn window_lists_one(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    run.keep(1, &Said::pasted(TEXTS[0]));
    run.open();
    Ok(())
}

#[given("the archive is turned off in the settings")]
fn archive_off(world: &mut IntegrationWorld) -> Result<()> {
    run(world)?.archive_off = true;
    Ok(())
}

#[given("the archive is on and holds no dictation")]
fn archive_empty(world: &mut IntegrationWorld) -> Result<()> {
    run(world)?;
    Ok(())
}

#[given("the archive holds two dictations with text and one damaged entry between them")]
fn damaged_between(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    run.keep(1, &Said::pasted(TEXTS[0]));
    run.damage(DAMAGED_SECOND);
    run.keep(3, &Said::pasted(TEXTS[1]));
    Ok(())
}
