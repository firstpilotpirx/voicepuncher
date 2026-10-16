use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use regex::Regex;

use voicepuncher_e2e_test::speech::{
    dropped_phrase_len, missing_word_share, repeated_phrases, synthesize, write_wav, Condition,
    Lang,
};

use crate::steps::file_transcription::{
    env, sut_bin, write_silence_wav, NO_CONFIG, STARTUP_FAILURE_EXIT_CODE,
};
use crate::steps::off_thread;
use crate::E2eWorld;

mod prompt;

const MAX_STARTUP_FAILURE: Duration = Duration::from_secs(3);
const REMOTE_SERVICE_URL: &str = "http://192.168.1.10:8081/v1";
const START_COMMAND: &str = "just start-model-servers";
/// A chunk seam may lose a word or two; more means the merge dropped real speech.
const MAX_MISSING_WORD_SHARE: f64 = 0.10;
/// Words the service may mishear in place, but not a whole phrase it skips after a pause.
const MAX_DROPPED_PHRASE_WORDS: usize = 5;
const OUTDATED_CONFIG: &str = "[whisper]\nmodel_path = \"/models/ggml-large-v3-turbo.bin\"\n";
const FIXED_SILENCE_LEVEL_CONFIG: &str = "[transcription]\nvad_rms_threshold = 0.01\n";
const AUTO_LANGUAGE_CONFIG: &str = "[transcription]\nlanguage = \"auto\"\n";

#[given("the speech recognition service is running")]
fn service_running(world: &mut E2eWorld) -> Result<()> {
    world.transcription_url = Some(env("VOICEPUNCHER_TRANSCRIPTION_URL")?);
    Ok(())
}

#[given("no speech recognition model is on disk for the app")]
fn no_model_for_app(world: &mut E2eWorld) {
    world.config = None;
}

#[given("the speech recognition service address is on another host")]
fn remote_service(world: &mut E2eWorld) {
    world.transcription_url = Some(REMOTE_SERVICE_URL.to_owned());
}

#[given("no speech recognition service is listening at the configured address")]
fn closed_service_port(world: &mut E2eWorld) -> Result<()> {
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    world.transcription_url = Some(format!("http://127.0.0.1:{port}/v1"));
    Ok(())
}

#[given("the configuration sets a speech recognition model file path")]
fn outdated_config(world: &mut E2eWorld) -> Result<()> {
    let dir = world.workdir()?;
    let path = dir.join("config.toml");
    std::fs::write(&path, OUTDATED_CONFIG)?;
    world.config = Some(path);
    Ok(())
}

#[given("the configuration sets a fixed silence level for chunking")]
fn fixed_silence_level_config(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("config.toml");
    std::fs::write(&path, FIXED_SILENCE_LEVEL_CONFIG)?;
    world.config = Some(path);
    Ok(())
}

#[given("the service detects the spoken language automatically")]
fn auto_language(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("config.toml");
    std::fs::write(&path, AUTO_LANGUAGE_CONFIG)?;
    world.config = Some(path);
    Ok(())
}

#[when("the user starts the app")]
async fn start_app(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let recording = world.workdir()?.join("silence_1s.wav");
        write_silence_wav(&recording, 1)?;
        let mut cmd = Command::new(sut_bin());
        match &world.config {
            Some(config) => cmd.arg("--config").arg(config),
            None => cmd.args(["--config", NO_CONFIG]),
        };
        cmd.arg("--input-file").arg(&recording).arg("--no-llm");
        if let Some(url) = &world.transcription_url {
            cmd.args(["--transcription-url", url]);
        }
        let started = Instant::now();
        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn {:?}", cmd.get_program()))?;
        world.elapsed = Some(started.elapsed());
        world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        world.exit_status = Some(output.status);
        Ok(())
    })
    .await
}

#[then("the app exits before recording anything and explains that only a local service is allowed")]
fn refused_remote(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    expect_message(world, "not on this machine")
}

#[then(
    "the app exits within 3 seconds with a message naming the address and how to start the service"
)]
fn reported_unreachable(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    let elapsed = world.elapsed.context("the app did not run")?;
    if elapsed >= MAX_STARTUP_FAILURE {
        bail!("the app took {elapsed:?} to exit");
    }
    let url = world
        .transcription_url
        .clone()
        .context("no service address")?;
    expect_message(world, &url)?;
    expect_message(world, START_COMMAND)
}

#[then("the app exits with a message saying the model is now chosen when starting the service and naming the service address setting")]
fn reported_outdated(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    expect_message(world, "starting the speech recognition service")?;
    expect_message(world, "[transcription].base_url")
}

#[then("the app exits with a message saying pauses now follow the recording's background noise and the setting can be deleted")]
fn reported_fixed_silence_level(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    expect_message(world, "transcription.vad_rms_threshold")?;
    expect_message(world, "background noise")?;
    expect_message(world, "delete")
}

#[then("no text is produced")]
fn no_text(world: &mut E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if status.code() == Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "the app exited with a startup failure; stderr: {}",
            world.stderr
        );
    }
    if !status.success() {
        bail!(
            "the app reported a failure ({status}); stderr: {}",
            world.stderr
        );
    }
    if !world.stdout.is_empty() {
        bail!("expected no text, got: {:?}", world.stdout);
    }
    Ok(())
}

pub fn expect_startup_failure(world: &E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if status.code() != Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "expected a startup failure, got {status}; stderr: {}",
            world.stderr
        );
    }
    Ok(())
}

pub fn expect_message(world: &E2eWorld, fragment: &str) -> Result<()> {
    if !world.stderr.contains(fragment) {
        bail!(
            "expected '{fragment}' in the app's message, got: {}",
            world.stderr
        );
    }
    Ok(())
}

#[given(regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold$")]
fn long_speech_recording(world: &mut E2eWorld, language: String) -> Result<()> {
    record_speech(world, language_of(&language)?, Condition::Normal, 0.0)
}

#[then("the recording is transcribed in chunks")]
fn transcribed_in_chunks(world: &mut E2eWorld) -> Result<()> {
    let chunks = Regex::new(r"file mode: (\d+) chunks")?
        .captures(&world.stderr)
        .and_then(|c| c[1].parse::<usize>().ok());
    match chunks {
        Some(n) if n >= 2 => Ok(()),
        _ => bail!(
            "the recording was not transcribed in chunks — stderr:\n{}",
            world.stderr
        ),
    }
}

#[then("the printed text repeats no phrase of the speech")]
fn no_repeated_phrase(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let repeated = repeated_phrases(spoken, &world.stdout);
    if !repeated.is_empty() {
        bail!(
            "phrases repeated at a chunk seam: {repeated:?}\n  printed: {}",
            world.stdout
        );
    }
    Ok(())
}

#[then("the printed text misses at most one spoken word in ten")]
fn few_missing_words(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let missing = missing_word_share(spoken, &world.stdout);
    if missing > MAX_MISSING_WORD_SHARE {
        bail!(
            "{:.0} % of the spoken words are missing\n  printed: {}",
            missing * 100.0,
            world.stdout
        );
    }
    Ok(())
}

#[then("no run of more than five consecutive spoken words is missing from the printed text")]
fn no_dropped_phrase(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let dropped = dropped_phrase_len(spoken, &world.stdout);
    if dropped > MAX_DROPPED_PHRASE_WORDS {
        bail!(
            "a phrase of {dropped} spoken words is missing\n  printed: {}",
            world.stdout
        );
    }
    Ok(())
}

#[given(
    regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold, ([^,]+)$"
)]
fn long_speech_under_condition(
    world: &mut E2eWorld,
    language: String,
    condition: String,
) -> Result<()> {
    record_speech(
        world,
        language_of(&language)?,
        condition_of(&condition)?,
        0.0,
    )
}

#[given(
    regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold, ([^,]+), after a lead-in of ([0-9.]+) seconds$"
)]
fn long_speech_after_lead_in(
    world: &mut E2eWorld,
    language: String,
    condition: String,
    lead_in_secs: f32,
) -> Result<()> {
    record_speech(
        world,
        language_of(&language)?,
        condition_of(&condition)?,
        lead_in_secs,
    )
}

fn condition_of(name: &str) -> Result<Condition> {
    Ok(match name {
        "spoken at a normal pace" => Condition::Normal,
        "spoken slowly" => Condition::Slow,
        "spoken fast" => Condition::Fast,
        "with pauses between sentences" => Condition::SentencePauses,
        "with pauses between words" => Condition::WordPauses,
        "with low background noise" => Condition::NoiseLow,
        "with high background noise" => Condition::NoiseHigh,
        "through a phone-quality line" => Condition::PhoneBand,
        "spoken quietly" => Condition::Quiet,
        "clipped" => Condition::Clipped,
        "with short audio dropouts" => Condition::Dropouts,
        other => bail!("no recording condition '{other}'"),
    })
}

fn language_of(name: &str) -> Result<Lang> {
    match name {
        "Russian" => Ok(Lang::Ru),
        "English" => Ok(Lang::En),
        other => bail!("no test speech in {other}"),
    }
}

/// The lead-in moves the chunk seams onto other words.
fn record_speech(
    world: &mut E2eWorld,
    lang: Lang,
    condition: Condition,
    lead_in_secs: f32,
) -> Result<()> {
    let samples = synthesize(lang, condition, lead_in_secs)?;
    let path = world.workdir()?.join("speech.wav");
    write_wav(&path, &samples)?;
    world.recording = Some(path);
    world.spoken = Some(lang);
    Ok(())
}

/// 2.3 s at 16 kHz: a command-length utterance, and the length that crashed whisper-server
/// 1.9.5 when it computed language probabilities for audio without speech.
const SILENT_UTTERANCE_SAMPLES: usize = 36_800;
const UTTERANCE_SAMPLE_RATE: u32 = 16_000;
/// As long as the command spotting service waited for a confirmation before it had a deadline.
const UTTERANCE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

fn confirm_silence(url: &str) -> std::result::Result<usize, String> {
    voicepuncher_command_spotting::confirm::UtteranceTranscriber::new(url)
        .transcribe(
            &[0.0; SILENT_UTTERANCE_SAMPLES],
            UTTERANCE_SAMPLE_RATE,
            UTTERANCE_DEADLINE,
        )
        .map(|heard| heard.words.len())
        .map_err(|e| format!("{e:#}"))
}

#[when("the command spotting service asks it to confirm an utterance that holds no speech")]
async fn confirm_utterance_without_speech(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = world
            .local_services
            .as_mut()
            .context("the speech recognition service was not started by a Given step")?;
        services.heard_words = Some(confirm_silence(&services.transcription_url()));
        Ok(())
    })
    .await
}

#[then("no words are heard and the service answers the next request")]
async fn no_words_and_service_answers(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = world
            .local_services
            .as_ref()
            .context("the speech recognition service was not started by a Given step")?;
        match services
            .heard_words
            .as_ref()
            .context("nothing was confirmed")?
        {
            Ok(0) => {}
            Ok(words) => bail!("{words} words were heard in an utterance without speech"),
            Err(e) => bail!("the confirmation of an utterance without speech failed: {e}"),
        }
        if let Err(e) = confirm_silence(&services.transcription_url()) {
            bail!("the service does not answer the next request: {e}");
        }
        Ok(())
    })
    .await
}
