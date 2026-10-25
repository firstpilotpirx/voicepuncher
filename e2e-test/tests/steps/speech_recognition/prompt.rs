//! The configured prompt of dictation: a domain term it names is written as the prompt
//! writes it, in every window of a long chunk, and no text of one request reaches another.

use cucumber::{given, then};
use std::fmt::Write as _;
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_e2e_test::conversions::saturating_usize;
use voicepuncher_e2e_test::machine_load;
use voicepuncher_e2e_test::speech::{write_wav, Lang};
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};
use voicepuncher_e2e_test::unanswered_copies::load_beside;

use crate::steps::file_transcription::transcribe;
use crate::E2eWorld;

/// A term whisper writes in Cyrillic ("кубернетис") when it hears it in Russian speech.
const DOMAIN_TERM: &str = "Kubernetes";
/// A phrase in the language of the speech that names the terms: large-v3-turbo ignores a bare
/// list of English terms in Russian speech ("Kubernetes, Grafana, …" still gave "Кубернетис"
/// with every whisper-server 1.9.5 setting), and writes the term as this phrase does.
const PROMPT: &str = "Мы развернули сервис в Kubernetes, смотрим метрики в Grafana.";
const RUSSIAN_VOICE: &str = "Milena";
const SENTENCE_WITH_TERM: &str = "Сегодня мы развернули новый сервис в кубернетис.";
/// Speech before the term in a long chunk, and after it: at least this long, so the term falls
/// after the first 30-second window and not into the last 5 s of the chunk, where whisper.cpp
/// drops the prompt.
const SECS_AROUND_TERM: f32 = 31.0;
const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// Far above any recording of a scenario: the app sends it as one chunk.
const ONE_CHUNK_THRESHOLD_SECS: u32 = 600;
const FIRST_SENTENCE: &str = "Вчера вечером бабушка испекла вишнёвый пирог для соседей.";
const UNRELATED_SENTENCE: &str = "Поезд до Казани отправляется с третьей платформы.";

#[given("the configured prompt names a domain term that speech recognition misses without it")]
fn prompt_names_term(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("config.toml");
    std::fs::write(&path, format!("[transcription]\nprompt = \"{PROMPT}\"\n"))?;
    world.config = Some(path);
    Ok(())
}

#[given("a recording is transcribed as one chunk however long it is")]
fn one_chunk(world: &mut E2eWorld) -> Result<()> {
    let path = world
        .config
        .clone()
        .context("no configuration was written")?;
    let mut config = std::fs::read_to_string(&path)?;
    let _ = writeln!(config, "chunk_threshold_secs = {ONE_CHUNK_THRESHOLD_SECS}");
    std::fs::write(&path, config)?;
    Ok(())
}

#[given("a recording file of a Russian sentence that says the term")]
fn sentence_with_term(world: &mut E2eWorld) -> Result<()> {
    let samples = russian(SENTENCE_WITH_TERM)?;
    record(world, "term.wav", &samples)
}

#[given(
    "a recording file of Russian speech that says the term only after its first 30 seconds and goes on after it"
)]
fn term_after_first_window(world: &mut E2eWorld) -> Result<()> {
    let filler = russian_speech_of(SECS_AROUND_TERM)?;
    let mut samples = filler.clone();
    samples.extend(russian(SENTENCE_WITH_TERM)?);
    samples.extend(filler);
    record(world, "long_term.wav", &samples)
}

#[given("the user transcribed a recording file of a Russian sentence")]
fn transcribed_first_sentence(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("first.wav");
    write_wav(&path, &russian(FIRST_SENTENCE)?)?;
    let output = transcribe(&path, world.config.as_deref())?;
    if !output.status.success() {
        let log = String::from_utf8_lossy(&output.stderr);
        let load = load_beside(&log, machine_load::report).unwrap_or_default();
        bail!(
            "the first dictation failed ({}): {log}\n{load}",
            output.status
        );
    }
    Ok(())
}

#[given("a recording file of an unrelated Russian sentence")]
fn unrelated_sentence(world: &mut E2eWorld) -> Result<()> {
    let samples = russian(UNRELATED_SENTENCE)?;
    record(world, "unrelated.wav", &samples)
}

#[then("the printed text holds the term as written in the prompt")]
fn term_as_in_prompt(world: &mut E2eWorld) -> Result<()> {
    if !world.app.stdout.contains(DOMAIN_TERM) {
        bail!(
            "the printed text lacks {DOMAIN_TERM:?}: {:?}\nstderr:\n{}",
            world.app.stdout,
            world.app.stderr
        );
    }
    Ok(())
}

#[then("the printed text holds no word of the first sentence that the second one does not say")]
fn no_echo(world: &mut E2eWorld) -> Result<()> {
    let second = words(UNRELATED_SENTENCE);
    let printed = words(&world.app.stdout);
    let echoed: Vec<String> = words(FIRST_SENTENCE)
        .into_iter()
        .filter(|w| !second.contains(w) && printed.contains(w))
        .collect();
    if !echoed.is_empty() {
        bail!(
            "the second dictation echoes {echoed:?} of the first: {:?}",
            world.app.stdout
        );
    }
    Ok(())
}

fn russian(text: &str) -> Result<Vec<i16>> {
    synthesized(&SayRequest { voice: Some(RUSSIAN_VOICE), words_per_minute: None, text })
}

/// Russian speech without the term, at least `secs` long.
fn russian_speech_of(secs: f32) -> Result<Vec<i16>> {
    let mut samples = Vec::new();
    while samples.len() < saturating_usize(secs * SAMPLE_RATE_HZ) {
        samples.extend(russian(Lang::Ru.text())?);
    }
    Ok(samples)
}

fn record(world: &mut E2eWorld, name: &str, samples: &[i16]) -> Result<()> {
    let path = world.workdir()?.join(name);
    write_wav(&path, samples)?;
    world.recording = Some(path);
    Ok(())
}

/// Lower-case words without punctuation, `ё` read as `е`.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace('ё', "е")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}
