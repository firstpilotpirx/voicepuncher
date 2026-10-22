//! Steps for text refinement answers the app must or must not paste.

use anyhow::{bail, Result};
use cucumber::{given, then, when};
use voicepuncher_e2e_test::speech::Lang;

use super::normalization::refine_with_live_service;
use super::requests::expect_raw_transcript_pasted;
use super::{answer, expect_pasted, use_stub};
use crate::steps::off_thread;
use crate::E2eWorld;
use voicepuncher_e2e_test::refinement_service_stub::StubBehavior;

const TRANSLATION_EN: &str = "We deploy the service through Docker every Friday evening.";
const UNRELATED_ANSWER: &str =
    "Dogs and foxes are both mammals that live in many parts of the world.";
const NORMALIZED_RU: &str = "Мы деплоим сервис через Docker каждую пятницу вечером!";
const RESPELLED_DICTATION: &str = "поставь о май зш и тему пауэрлевел десять к в вс код";
const RESPELLED_ANSWER: &str = "Поставь oh-my-zsh и тему Powerlevel10k в VS Code.";
const MIXED_DICTATION: &str = "Сделай code review этого pull request до вечера";
const MIXED_TRANSLATED: &str = "Make a code review of this pull request before evening.";
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

#[given("the text refinement service answers a short Russian dictation with its English words and IT terms respelled in Latin script, so most of its letters are Latin")]
fn service_respells(world: &mut E2eWorld) -> Result<()> {
    world.dictated = Some(RESPELLED_DICTATION);
    use_stub(world, answer(RESPELLED_ANSWER.to_owned()))
}

#[given("the text refinement service answers a Russian sentence with English words in it by translating its Russian words into English")]
fn service_translates_russian_part(world: &mut E2eWorld) -> Result<()> {
    world.dictated = Some(MIXED_DICTATION);
    use_stub(world, answer(MIXED_TRANSLATED.to_owned()))
}

#[when("the user dictates that sentence with refinement")]
async fn dictate_that_sentence(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let dictated = world.dictated.unwrap_or_default();
        refine_with_live_service(world, dictated)
    })
    .await
}

#[then("the service's answer is pasted, not the raw transcript of that sentence")]
fn respelled_answer_pasted(world: &mut E2eWorld) -> Result<()> {
    if world.stdout != RESPELLED_ANSWER {
        bail!(
            "expected {RESPELLED_ANSWER:?} to be pasted, got {:?}",
            world.stdout
        );
    }
    Ok(())
}

#[then("the raw transcript of that sentence is pasted")]
fn dictated_sentence_pasted(world: &mut E2eWorld) -> Result<()> {
    let dictated = world.dictated.unwrap_or_default();
    if world.stdout != dictated {
        bail!(
            "expected the raw {dictated:?} to be pasted, got {:?}",
            world.stdout
        );
    }
    Ok(())
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
