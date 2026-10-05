//! Steps that send a transcript through the app's refine-or-raw path and the live text
//! refinement service, then judge the normalization that would be pasted.

use anyhow::{bail, Context, Result};
use cucumber::{then, when};
use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::config::RefinementCfg;
use voicepuncher::core::application::text_refiner::refine_or_raw;
use voicepuncher::core::domain::RawTranscript;

use crate::E2eWorld;

const UNPUNCTUATED_RU: &str = "я думаю что это хорошая идея но надо обсудить с командой";
const CLAUSE_MARKS: [char; 3] = [',', ':', ';'];
const SENTENCE_MARKS: [char; 3] = ['.', '!', '?'];
const WITH_FILLERS_EN: &str = "I think um we should uh postpone the release";
const FILLER_SOUNDS: [&str; 2] = ["um", "uh"];
const WITHOUT_FILLERS_EN: [&str; 7] = ["i", "think", "we", "should", "postpone", "the", "release"];
const CYRILLIC_TERM_RU: &str = "мы деплоим это через докер каждую пятницу";
const TERM_IN_LATIN: &str = "Docker";
const TERM_SENTENCE_RU_WORDS: [&str; 2] = ["деплоим", "пятницу"];
const MIXED_RU_EN: &str = "Сделай code review этого pull request до вечера";
const MIXED_KEPT: [&str; 4] = ["сделай", "code review", "pull request", "вечера"];
const DISAGREEING_VERBS_RU: &str = "мама сказал что она купил новый телефон";
const AGREEING_WORDS_RU: [&str; 7] = [
    "мама",
    "сказала",
    "что",
    "она",
    "купила",
    "новый",
    "телефон",
];

/// Refines `transcript` exactly as the app does before pasting it.
pub fn refine_with_live_service(world: &mut E2eWorld, transcript: &str) -> Result<()> {
    let mut refiner = live_refiner(world)?;
    let raw = RawTranscript::new(transcript.to_owned())?;
    let pasted = refine_or_raw(&mut refiner, &raw).context("nothing would be pasted")?;
    world.stdout = pasted.as_str().to_owned();
    Ok(())
}

pub fn live_refiner(world: &E2eWorld) -> Result<OpenAiTextRefiner> {
    let base_url = world
        .refinement_url
        .clone()
        .context("no text refinement service")?;
    OpenAiTextRefiner::connect(RefinementCfg {
        base_url,
        ..RefinementCfg::default()
    })
}

#[when("the user dictates a sentence that the speech recognition service returns without punctuation or capitals")]
fn dictate_unpunctuated(world: &mut E2eWorld) -> Result<()> {
    refine_with_live_service(world, UNPUNCTUATED_RU)
}

#[when("the user dictates a sentence with hesitation sounds in it")]
fn dictate_with_fillers(world: &mut E2eWorld) -> Result<()> {
    refine_with_live_service(world, WITH_FILLERS_EN)
}

#[when("the user dictates in Russian a sentence naming a tool whose name the speech recognition service wrote in Cyrillic")]
fn dictate_cyrillic_term(world: &mut E2eWorld) -> Result<()> {
    refine_with_live_service(world, CYRILLIC_TERM_RU)
}

#[when("the user dictates a Russian sentence with English words in it")]
fn dictate_mixed(world: &mut E2eWorld) -> Result<()> {
    refine_with_live_service(world, MIXED_RU_EN)
}

#[when(
    "the user dictates in Russian a sentence whose verbs do not agree with their subject in gender"
)]
fn dictate_disagreeing_verbs(world: &mut E2eWorld) -> Result<()> {
    refine_with_live_service(world, DISAGREEING_VERBS_RU)
}

#[then("the pasted text starts with a capital letter, separates its clauses with punctuation and ends with a sentence mark")]
fn punctuated(world: &mut E2eWorld) -> Result<()> {
    let pasted = world.stdout.as_str();
    let capital = pasted.chars().next().is_some_and(char::is_uppercase);
    let clauses = pasted.contains(CLAUSE_MARKS);
    let ends = pasted.ends_with(SENTENCE_MARKS);
    if !(capital && clauses && ends) {
        bail!("expected a capital, clause punctuation and a sentence mark: {pasted:?}");
    }
    Ok(())
}

#[then("the pasted text has no hesitation sounds and keeps every other word")]
fn fillers_removed(world: &mut E2eWorld) -> Result<()> {
    let words = lowercase_words(&world.stdout);
    if let Some(filler) = FILLER_SOUNDS
        .iter()
        .find(|f| words.contains(&(**f).to_owned()))
    {
        bail!(
            "the hesitation sound '{filler}' was pasted: {:?}",
            world.stdout
        );
    }
    if words != WITHOUT_FILLERS_EN {
        bail!("expected exactly the words {WITHOUT_FILLERS_EN:?}, got {words:?}");
    }
    Ok(())
}

#[then("the pasted text names the tool in its usual Latin spelling and keeps the rest in Russian")]
fn term_in_latin(world: &mut E2eWorld) -> Result<()> {
    if !world.stdout.contains(TERM_IN_LATIN) {
        bail!("expected '{TERM_IN_LATIN}' in {:?}", world.stdout);
    }
    expect_contains_all(&world.stdout, &TERM_SENTENCE_RU_WORDS)
}

#[then("the pasted text keeps the Russian words in Russian and the English words in English")]
fn mixed_kept(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &MIXED_KEPT)
}

#[then("the pasted text has those verbs in the form that agrees with the subject and keeps the same words in the same order")]
fn endings_agree(world: &mut E2eWorld) -> Result<()> {
    let words = lowercase_words(&world.stdout);
    if words != AGREEING_WORDS_RU {
        bail!("expected exactly the words {AGREEING_WORDS_RU:?}, got {words:?}");
    }
    Ok(())
}

fn expect_contains_all(pasted: &str, expected: &[&str]) -> Result<()> {
    let lowercase = pasted.to_lowercase();
    match expected.iter().find(|text| !lowercase.contains(*text)) {
        Some(missing) => bail!("expected '{missing}' in the pasted text {pasted:?}"),
        None => Ok(()),
    }
}

fn lowercase_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}
