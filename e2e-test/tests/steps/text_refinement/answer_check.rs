//! Steps for text refinement answers the app must or must not paste.

use anyhow::{bail, Result};
use cucumber::{given, then};
use voicepuncher_e2e_test::speech::Lang;

use super::{answer, expect_pasted, expect_raw_transcript_pasted, use_stub};
use crate::steps::refinement_service_stub::StubBehavior;
use crate::E2eWorld;

const TRANSLATION_EN: &str = "We deploy the service through Docker every Friday evening.";
const UNRELATED_ANSWER: &str =
    "Dogs and foxes are both mammals that live in many parts of the world.";
const NORMALIZED_RU: &str = "Мы деплоим сервис через Docker каждую пятницу вечером!";
const REJECTION_LOG: &str = "refinement answer rejected";
/// A word of the spoken Russian sentence and the form the stub answers it with.
const SPOKEN_FORM: &str = "пятницу";
const CORRECTED_FORM: &str = "пятница";

#[given("the text refinement service answers a Russian transcript with its English translation")]
fn service_translates(world: &mut E2eWorld) -> Result<()> {
    world.spoken = Some(Lang::Ru);
    use_stub(world, answer(TRANSLATION_EN.to_owned()))
}

#[given("the text refinement service answers with text that shares few words with the transcript")]
fn service_answers_unrelated(world: &mut E2eWorld) -> Result<()> {
    use_stub(world, answer(UNRELATED_ANSWER.to_owned()))
}

#[given("the text refinement service answers with the transcript plus punctuation, capitals and one technical term respelled in Latin script")]
fn service_normalizes(world: &mut E2eWorld) -> Result<()> {
    world.spoken = Some(Lang::Ru);
    use_stub(world, answer(NORMALIZED_RU.to_owned()))
}

#[given("the text refinement service answers a short Russian transcript with the endings of most of its words corrected")]
fn service_corrects_endings(world: &mut E2eWorld) -> Result<()> {
    world.spoken = Some(Lang::Ru);
    use_stub(world, StubBehavior::CorrectEndings)
}

#[then("the raw transcript is pasted and the rejected answer is logged")]
fn raw_pasted_rejection_logged(world: &mut E2eWorld) -> Result<()> {
    expect_raw_transcript_pasted(world)?;
    let rejected = [TRANSLATION_EN, UNRELATED_ANSWER];
    if rejected.contains(&world.stdout.as_str()) {
        bail!("the rejected answer was pasted: {:?}", world.stdout);
    }
    let logged = world.stderr.contains(REJECTION_LOG)
        && rejected.iter().any(|text| world.stderr.contains(text));
    if !logged {
        bail!(
            "expected '{REJECTION_LOG}' and the answer in the log, got: {}",
            world.stderr
        );
    }
    Ok(())
}

#[then("the service's answer is pasted")]
fn normalization_pasted(world: &mut E2eWorld) -> Result<()> {
    expect_pasted(world, NORMALIZED_RU)
}

#[then("the service's answer is pasted, not the raw transcript")]
fn corrected_endings_pasted(world: &mut E2eWorld) -> Result<()> {
    let pasted = world.stdout.to_lowercase();
    if world.stderr.contains(REJECTION_LOG)
        || !pasted.contains(CORRECTED_FORM)
        || pasted.contains(SPOKEN_FORM)
    {
        bail!(
            "expected the answer with '{CORRECTED_FORM}' in place of '{SPOKEN_FORM}' to be pasted, got {:?}; stderr: {}",
            world.stdout,
            world.stderr
        );
    }
    Ok(())
}
