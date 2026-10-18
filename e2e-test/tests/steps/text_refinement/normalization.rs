//! Steps that send a transcript through the app's refine-or-raw path and the live text
//! refinement service, then judge the normalization that would be pasted.

use anyhow::{bail, Context, Result};
use cucumber::{then, when};
use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::config::RefinementCfg;
use voicepuncher::core::application::text_refiner::refine_or_raw;
use voicepuncher::core::domain::RawTranscript;

use crate::steps::off_thread;
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
const RETRY_RU: &str = "добавь ретрай для этого запроса";
const RETRY_EN: [&str; 1] = ["retry"];
const RETRY_RU_WORDS: [&str; 2] = ["добавь", "запроса"];
const TWO_VALUES_RU: &str = "у этого флага всего два значения энейблд и дизейблд";
const TWO_VALUES_EN: [&str; 2] = ["enabled", "disabled"];
const MISHEARD_DEBUG_RU: &str =
    "сервис пишет слишком мало логов нужен уровень дибаг чтобы найти ошибку";
const MISHEARD_DEBUG_EN: [&str; 1] = ["debug"];
const MISHEARD_DEBUG_HEARD: &str = "дибаг";
const LOANWORDS_RU: &str = "менеджер попросил перезагрузить компьютер и проверить интернет";
const LOANWORDS_KEPT: [&str; 3] = ["менеджер", "компьютер", "интернет"];
const RUSSIFIED_VERBS_RU: &str = "мы деплоим релиз завтра а потом закомитим фикс";
const RUSSIFIED_VERBS_KEPT: [&str; 2] = ["деплои", "закомит"];
const SHELL_SYLLABLES_RU: &str = "для баша есть фреймворк баш ит с темой пауэр лайн";
const SHELL_TERMS: [&str; 2] = ["bash-it", "powerline"];
const SHELL_SENTENCE_RU_WORDS: [&str; 2] = ["фреймворк", "темой"];
const LANGUAGE_SYLLABLES_RU: &str = "он переписал весь проект с джава скрипт на тайп скрипт";
const LANGUAGE_TERMS: [&str; 2] = ["JavaScript", "TypeScript"];
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
async fn dictate_unpunctuated(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, UNPUNCTUATED_RU)
    })
    .await
}

#[when("the user dictates a sentence with hesitation sounds in it")]
async fn dictate_with_fillers(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, WITH_FILLERS_EN)
    })
    .await
}

#[when("the user dictates in Russian a sentence naming a tool whose name the speech recognition service wrote in Cyrillic")]
async fn dictate_cyrillic_term(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, CYRILLIC_TERM_RU)
    })
    .await
}

#[when("the user dictates a Russian sentence with English words in it")]
async fn dictate_mixed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, MIXED_RU_EN)
    })
    .await
}

#[when(
    "the user dictates in Russian a sentence whose verbs do not agree with their subject in gender"
)]
async fn dictate_disagreeing_verbs(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, DISAGREEING_VERBS_RU)
    })
    .await
}

#[when("the user dictates in Russian a request to add a retry that speech recognition writes in Cyrillic")]
async fn dictate_retry(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, RETRY_RU)
    })
    .await
}

#[when("the user dictates in Russian that a flag has only two values, enabled and disabled, and speech recognition writes both in Cyrillic")]
async fn dictate_two_values(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, TWO_VALUES_RU)
    })
    .await
}

#[when("the user dictates in Russian that a service needs the debug log level and speech recognition misheard the word debug")]
async fn dictate_misheard_debug(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, MISHEARD_DEBUG_RU)
    })
    .await
}

#[when(
    "the user dictates in Russian a sentence with the loanwords for manager, computer and internet"
)]
async fn dictate_loanwords(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, LOANWORDS_RU)
    })
    .await
}

#[when(
    "the user dictates in Russian a sentence with English verb stems that carry Russian endings"
)]
async fn dictate_russified_verbs(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, RUSSIFIED_VERBS_RU)
    })
    .await
}

#[when("the user dictates in Russian that Bash has the framework Bash-it with the Powerline theme, and speech recognition writes both as Cyrillic syllables")]
async fn dictate_shell_syllables(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, SHELL_SYLLABLES_RU)
    })
    .await
}

#[when("the user dictates in Russian that someone rewrote a project from JavaScript to TypeScript, and speech recognition writes both as Cyrillic syllables")]
async fn dictate_language_syllables(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        refine_with_live_service(world, LANGUAGE_SYLLABLES_RU)
    })
    .await
}

#[then("the pasted text has the English word retry in Latin script and keeps the rest in Russian")]
fn retry_in_latin(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &RETRY_EN)?;
    expect_contains_all(&world.stdout, &RETRY_RU_WORDS)
}

#[then("the pasted text has the English words enabled and disabled in Latin script")]
fn two_values_in_latin(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &TWO_VALUES_EN)
}

#[then("the pasted text has the English word debug in Latin script instead of the misheard word")]
fn debug_restored(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &MISHEARD_DEBUG_EN)?;
    if world.stdout.to_lowercase().contains(MISHEARD_DEBUG_HEARD) {
        bail!(
            "the misheard '{MISHEARD_DEBUG_HEARD}' was pasted: {:?}",
            world.stdout
        );
    }
    Ok(())
}

#[then("the pasted text has those loanwords in Cyrillic")]
fn loanwords_kept(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &LOANWORDS_KEPT)
}

#[then("the pasted text has those verbs in Cyrillic")]
fn russified_verbs_kept(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &RUSSIFIED_VERBS_KEPT)
}

#[then("the pasted text has Bash-it and Powerline in Latin script, ignoring letter case, and keeps the rest in Russian")]
fn shell_terms_restored(world: &mut E2eWorld) -> Result<()> {
    expect_contains_all(&world.stdout, &SHELL_TERMS)?;
    expect_contains_all(&world.stdout, &SHELL_SENTENCE_RU_WORDS)
}

#[then("the pasted text has JavaScript and TypeScript in Latin script")]
fn language_terms_restored(world: &mut E2eWorld) -> Result<()> {
    match LANGUAGE_TERMS
        .iter()
        .find(|term| !world.stdout.contains(*term))
    {
        Some(missing) => bail!("expected '{missing}' in the pasted text {:?}", world.stdout),
        None => Ok(()),
    }
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
