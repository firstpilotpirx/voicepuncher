//! The user's languages: with automatic detection a short phrase stays in the language it was
//! spoken in, never in a language the user does not speak.
//!
//! The real service cannot be made to take a phrase for Korean on demand, so the Korean
//! scenario proves the outcome: a short Russian phrase, the kind whisper took for Korean,
//! comes out in Cyrillic with no Hangul.

use cucumber::{given, then};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_e2e_test::speech::split_words;

use voicepuncher_e2e_test::speech::write_wav;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use crate::E2eWorld;

const USER_LANGUAGES_CONFIG: &str =
    "[transcription]\nlanguage = \"auto\"\nlanguages = [\"ru\", \"en\"]\n";
const RUSSIAN_VOICE: &str = "Milena";
const ENGLISH_VOICE: &str = "Eddy";
/// Short enough for whisper to take it for another language now and then.
const SHORT_RUSSIAN_PHRASE: &str = "Хорошо, давай.";
const SHORT_ENGLISH_PHRASE: &str = "Please restart the server.";

#[given(
    "the service detects the spoken language automatically and the user's languages are Russian and English"
)]
fn user_languages(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("config.toml");
    std::fs::write(&path, USER_LANGUAGES_CONFIG)?;
    world.config = Some(path);
    Ok(())
}

#[given(regex = r"^a recording file of a short (Russian|English) phrase$")]
fn short_phrase(world: &mut E2eWorld, language: String) -> Result<()> {
    let (voice, text) = match language.as_str() {
        "Russian" => (RUSSIAN_VOICE, SHORT_RUSSIAN_PHRASE),
        _ => (ENGLISH_VOICE, SHORT_ENGLISH_PHRASE),
    };
    let samples = synthesized(&SayRequest { voice: Some(voice), words_per_minute: None, text })?;
    let path = world.workdir()?.join("short_phrase.wav");
    write_wav(&path, &samples)?;
    world.recording = Some(path);
    Ok(())
}

#[then("the printed text is the phrase in Russian, with no Korean characters")]
fn russian_without_korean(world: &mut E2eWorld) -> Result<()> {
    let printed = &world.app.stdout;
    if printed.chars().any(is_hangul) {
        bail!("the printed text holds Korean characters: {printed:?}");
    }
    if printed
        .chars()
        .any(|c| c.is_alphabetic() && !is_cyrillic(c))
    {
        bail!("the printed text holds letters outside the Cyrillic alphabet: {printed:?}");
    }
    let spoken = split_words(SHORT_RUSSIAN_PHRASE);
    if !split_words(printed)
        .iter()
        .any(|word| spoken.contains(word))
    {
        bail!(
            "the printed text holds no word of {SHORT_RUSSIAN_PHRASE:?}: {printed:?}\nstderr:\n{}",
            world.app.stderr
        );
    }
    Ok(())
}

#[then("the printed text holds the English words of the phrase, in Latin letters")]
fn english_in_latin(world: &mut E2eWorld) -> Result<()> {
    let printed = &world.app.stdout;
    if printed.chars().any(|c| c.is_alphabetic() && !c.is_ascii()) {
        bail!("the printed text holds letters outside the Latin alphabet: {printed:?}");
    }
    let spoken = split_words(SHORT_ENGLISH_PHRASE);
    let heard = split_words(printed);
    let missing: Vec<&String> = spoken.iter().filter(|word| !heard.contains(word)).collect();
    if !missing.is_empty() {
        bail!("the printed text lacks {missing:?} of {SHORT_ENGLISH_PHRASE:?}: {printed:?}");
    }
    Ok(())
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}')
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{AC00}'..='\u{D7AF}')
}
