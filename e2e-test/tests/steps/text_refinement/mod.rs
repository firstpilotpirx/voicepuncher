use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use crate::steps::file_transcription::{env, sut_bin, write_silence_wav, NO_CONFIG};
use crate::steps::refinement_service_stub::{RefinementServiceStub, StubBehavior};
use crate::steps::speech_recognition::{expect_message, expect_startup_failure};
use crate::E2eWorld;
use voicepuncher_e2e_test::speech::Lang;

mod answer_check;
mod evaluation;
mod live;
mod normalization;

const MAX_STARTUP_FAILURE: Duration = Duration::from_secs(3);
const REMOTE_SERVICE_URL: &str = "http://192.168.1.10:8080/v1";
const START_COMMAND: &str = "just start-model-servers";
const OUTDATED_CONFIG: &str = "[llm]\nmodel_path = \"/models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf\"\n";
const OFFERED_MODELS: [&str; 2] = ["qwen2.5-7b-instruct", "llama-3.1-8b-instruct"];

const SPEECH: &str = "The quick brown fox jumps over the lazy dog.";
const SPEECH_WORDS: &str = "quick brown fox";
const TRANSLATION_REQUEST: &str = "Translate this sentence into Russian.";
const TRANSLATION_REQUEST_WORDS: [&str; 2] = ["translate", "russian"];
/// A normalization of `SPEECH` that its raw transcript never equals.
const CORRECTED_TEXT: &str = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG!";
/// Long enough (over half the transcript) that only the output limit can reject it.
const CUT_OFF_TEXT: &str = "The quick brown fox jumps over the";
const OUTPUT_LIMIT_LOG: &str = "output limit";
const REASONING_BLOCK: &str = "<think>\nThe transcript is a pangram; nothing to fix.\n</think>\n\n";
const TRANSCRIPT_OPEN: &str = "<transcript>\n";
const TRANSCRIPT_CLOSE: &str = "\n</transcript>";
const REFINEMENT_FAILURE_LOG: &str = "refinement failed";
const DEFAULT_VOICE: &str = "Samantha";
const RUSSIAN_VOICE: &str = "Milena";
const SPEECH_RU: &str = "Мы деплоим сервис через докер каждую пятницу вечером.";
const SPEECH_RU_WORDS: &str = "пятницу";

fn use_stub(world: &mut E2eWorld, behavior: StubBehavior) -> Result<()> {
    let stub = RefinementServiceStub::spawn(behavior)?;
    world.refinement_url = Some(stub.url.clone());
    world.refinement_stub = Some(stub);
    Ok(())
}

fn answer(content: String) -> StubBehavior {
    StubBehavior::Answer {
        content,
        finish_reason: "stop",
    }
}

#[given("the text refinement service answers with corrected text")]
fn service_answers(world: &mut E2eWorld) -> Result<()> {
    use_stub(world, answer(CORRECTED_TEXT.to_owned()))
}

#[given("the text refinement service answers with a reasoning block followed by corrected text")]
fn service_answers_with_reasoning(world: &mut E2eWorld) -> Result<()> {
    use_stub(world, answer(format!("{REASONING_BLOCK}{CORRECTED_TEXT}")))
}

#[given("the text refinement service stops generating at its output limit")]
fn service_cut_off(world: &mut E2eWorld) -> Result<()> {
    let behavior = StubBehavior::Answer {
        content: CUT_OFF_TEXT.to_owned(),
        finish_reason: "length",
    };
    use_stub(world, behavior)
}

#[given("the text refinement service stops after the app has started")]
fn service_stops(world: &mut E2eWorld) -> Result<()> {
    use_stub(world, StubBehavior::StopAfterStartupCheck)
}

#[given("the text refinement service offers only models other than the configured one")]
fn service_offers_other_models(world: &mut E2eWorld) -> Result<()> {
    use_stub(world, StubBehavior::OfferModels(OFFERED_MODELS.to_vec()))
}

#[given("the text refinement service is running")]
fn real_service_running(world: &mut E2eWorld) -> Result<()> {
    world.refinement_url = Some(env("VOICEPUNCHER_REFINEMENT_URL")?);
    Ok(())
}

#[given("no language model is on disk for the app")]
fn no_language_model_for_app(world: &mut E2eWorld) {
    world.config = None;
}

#[given("the text refinement service address is on another host")]
fn remote_refinement_service(world: &mut E2eWorld) {
    world.refinement_url = Some(REMOTE_SERVICE_URL.to_owned());
}

#[given("no text refinement service is listening at the configured address")]
fn closed_refinement_port(world: &mut E2eWorld) -> Result<()> {
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    world.refinement_url = Some(format!("http://127.0.0.1:{port}/v1"));
    Ok(())
}

#[given("the configuration sets a language model file path")]
fn outdated_llm_config(world: &mut E2eWorld) -> Result<()> {
    let path = world.workdir()?.join("config.toml");
    std::fs::write(&path, OUTDATED_CONFIG)?;
    world.config = Some(path);
    Ok(())
}

#[when(regex = r"^the user transcribes a recording of speech (with|without) refinement$")]
fn transcribe_speech(world: &mut E2eWorld, refinement: String) -> Result<()> {
    let recording = match world.spoken {
        Some(Lang::Ru) => voiced_speech_recording(world, RUSSIAN_VOICE, SPEECH_RU)?,
        _ => speech_recording(world, SPEECH)?,
    };
    run_app(world, &recording, refinement == "with")
}

#[when("the user dictates a sentence that asks for a translation")]
fn dictate_translation_request(world: &mut E2eWorld) -> Result<()> {
    let recording = speech_recording(world, TRANSLATION_REQUEST)?;
    run_app(world, &recording, true)
}

#[when("the user starts the app with refinement")]
fn start_app_with_refinement(world: &mut E2eWorld) -> Result<()> {
    let recording = world.workdir()?.join("silence_1s.wav");
    write_silence_wav(&recording, 1)?;
    run_app(world, &recording, true)
}

#[then("the text the service returned is pasted")]
fn service_text_pasted(world: &mut E2eWorld) -> Result<()> {
    expect_pasted(world, CORRECTED_TEXT)
}

#[then("only the corrected text is pasted")]
fn only_corrected_text_pasted(world: &mut E2eWorld) -> Result<()> {
    expect_pasted(world, CORRECTED_TEXT)
}

#[then("the text refinement service receives the sentence as the text to correct, apart from the correction instructions")]
fn sentence_sent_as_data(world: &mut E2eWorld) -> Result<()> {
    let request = only_chat_request(world)?;
    let messages = request["messages"]
        .as_array()
        .context("the request has no messages")?;
    let (Some(instruction), Some(data)) = (messages.first(), messages.last()) else {
        bail!("expected an instruction and a transcript message, got {messages:?}");
    };
    let instruction_text = expect_message_role(instruction, "system")?.to_lowercase();
    let data_text = expect_message_role(data, "user")?;
    let sentence = data_text
        .strip_prefix(TRANSCRIPT_OPEN)
        .and_then(|rest| rest.strip_suffix(TRANSCRIPT_CLOSE))
        .with_context(|| format!("the transcript is not delimited: {data_text:?}"))?
        .to_lowercase();
    for word in TRANSLATION_REQUEST_WORDS {
        if !sentence.contains(word) {
            bail!("expected '{word}' in the transcript sent to the service: {sentence:?}");
        }
    }
    if instruction_text.contains(&sentence) {
        bail!("the dictated sentence leaked into the instruction: {instruction_text:?}");
    }
    Ok(())
}

#[then("the raw transcript is pasted and the failure is logged")]
fn raw_pasted_failure_logged(world: &mut E2eWorld) -> Result<()> {
    expect_raw_transcript_pasted(world)?;
    if !world.stderr.contains(REFINEMENT_FAILURE_LOG) {
        bail!(
            "expected '{REFINEMENT_FAILURE_LOG}' in the log, got: {}",
            world.stderr
        );
    }
    Ok(())
}

#[then("the raw transcript is pasted instead of the cut-off text")]
fn raw_pasted_not_cut_off(world: &mut E2eWorld) -> Result<()> {
    only_chat_request(world)?;
    expect_raw_transcript_pasted(world)?;
    expect_message(world, OUTPUT_LIMIT_LOG)
}

#[then("the app starts and is ready to record")]
fn app_started_ready(world: &mut E2eWorld) -> Result<()> {
    expect_success(world)
}

#[then("the raw transcript is pasted")]
fn raw_pasted(world: &mut E2eWorld) -> Result<()> {
    expect_raw_transcript_pasted(world)
}

#[then("the app exits before recording anything and explains that only a local text refinement service is allowed")]
fn refused_remote_refinement(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    expect_message(world, "refinement.base_url")?;
    expect_message(world, "not on this machine")
}

#[then("the app exits within 3 seconds with a message naming the text refinement service address and how to start it")]
fn reported_refinement_unreachable(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    let elapsed = world.elapsed.context("the app did not run")?;
    if elapsed >= MAX_STARTUP_FAILURE {
        bail!("the app took {elapsed:?} to exit");
    }
    let url = world.refinement_url.clone().context("no service address")?;
    expect_message(world, &url)?;
    expect_message(world, START_COMMAND)
}

#[then("the app exits with a message listing the models the service offers")]
fn reported_offered_models(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    OFFERED_MODELS
        .iter()
        .try_for_each(|model| expect_message(world, model))
}

#[then("the app exits with a message saying the language model is now chosen when starting the text refinement service and naming its address setting")]
fn reported_outdated_llm_config(world: &mut E2eWorld) -> Result<()> {
    expect_startup_failure(world)?;
    expect_message(world, "starting the text refinement service")?;
    expect_message(world, "[refinement].base_url")
}

fn speech_recording(world: &mut E2eWorld, text: &str) -> Result<PathBuf> {
    voiced_speech_recording(world, DEFAULT_VOICE, text)
}

/// Synthesizes `text` with the macOS voice `voice` into the scenario's recording.
fn voiced_speech_recording(world: &mut E2eWorld, voice: &str, text: &str) -> Result<PathBuf> {
    let path = world.workdir()?.join("speech.wav");
    let status = Command::new("say")
        .args([
            "-v",
            voice,
            "--file-format=WAVE",
            "--data-format=LEI16@16000",
            "-o",
        ])
        .arg(&path)
        .arg(text)
        .status()
        .context("failed to run macOS 'say'")?;
    if !status.success() {
        bail!("'say' failed to synthesize speech ({status})");
    }
    Ok(path)
}

fn run_app(world: &mut E2eWorld, recording: &Path, with_refinement: bool) -> Result<()> {
    let mut cmd = Command::new(sut_bin());
    match &world.config {
        Some(config) => cmd.arg("--config").arg(config),
        None => cmd.args(["--config", NO_CONFIG]),
    };
    cmd.arg("--input-file").arg(recording);
    cmd.args([
        "--transcription-url",
        &env("VOICEPUNCHER_TRANSCRIPTION_URL")?,
    ]);
    if let Some(url) = &world.refinement_url {
        cmd.args(["--refinement-url", url]);
    }
    if !with_refinement {
        cmd.arg("--no-llm");
    }
    let started = Instant::now();
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn {:?}", cmd.get_program()))?;
    world.elapsed = Some(started.elapsed());
    world.stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    world.exit_status = Some(output.status);
    Ok(())
}

fn only_chat_request(world: &E2eWorld) -> Result<serde_json::Value> {
    let stub = world
        .refinement_stub
        .as_ref()
        .context("no text refinement service stub")?;
    let requests = stub.chat_requests();
    let [request] = requests.as_slice() else {
        bail!(
            "expected one request to the text refinement service, got {}",
            requests.len()
        );
    };
    serde_json::from_str(request).context("the request to the service is not JSON")
}

fn expect_message_role<'a>(message: &'a serde_json::Value, role: &str) -> Result<&'a str> {
    if message["role"] != role {
        bail!("expected a '{role}' message, got {message}");
    }
    message["content"]
        .as_str()
        .with_context(|| format!("the '{role}' message has no text"))
}

fn expect_success(world: &E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if !status.success() {
        bail!("the app failed with {status}; stderr: {}", world.stderr);
    }
    Ok(())
}

fn expect_pasted(world: &E2eWorld, expected: &str) -> Result<()> {
    expect_success(world)?;
    if world.stdout != expected {
        bail!(
            "expected {expected:?} to be pasted, got {:?}; stderr: {}",
            world.stdout,
            world.stderr
        );
    }
    Ok(())
}

fn expect_raw_transcript_pasted(world: &E2eWorld) -> Result<()> {
    expect_success(world)?;
    let pasted = world.stdout.to_lowercase();
    let (spoken, spoken_words) = match world.spoken {
        Some(Lang::Ru) => (SPEECH_RU, SPEECH_RU_WORDS),
        _ => (SPEECH, SPEECH_WORDS),
    };
    if !pasted.contains(spoken_words) || world.stdout == CORRECTED_TEXT {
        bail!(
            "expected the raw transcript of {spoken:?} to be pasted, got {:?}",
            world.stdout
        );
    }
    if world.stdout == CUT_OFF_TEXT {
        bail!("the cut-off text was pasted");
    }
    Ok(())
}
