use std::net::{TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use regex::Regex;
use voicepuncher_copy_pool::ServiceAddress;

use voicepuncher_e2e_test::speech::{
    dropped_phrase_len, missing_word_share, repeated_phrases, synthesize, write_wav, Condition,
    Lang,
};

use crate::steps::file_transcription::{
    env, sut_bin, write_silence_wav, NO_CONFIG, STARTUP_FAILURE_EXIT_CODE,
};
use crate::steps::local_services::{Role, ScenarioServices};
use crate::steps::off_thread;
use crate::E2eWorld;

mod languages;
mod prompt;

mod chunked_speech;
mod service_models;

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
    // The app looks for copies on the ports above the address too: something else of the
    // machine listening on one of them would be taken for a copy of the service
    loop {
        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        let url = format!("http://127.0.0.1:{port}/v1");
        let copy_ports = ServiceAddress::parse(&url)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .copy_ports();
        if !std::iter::once(port).chain(copy_ports).any(answers) {
            world.transcription_url = Some(url);
            return Ok(());
        }
    }
}

fn answers(port: u16) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
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
        cmd.arg("--input-file").arg(&recording);
        if let Some(url) = &world.transcription_url {
            cmd.args(["--transcription-url", url]);
        }
        let started = Instant::now();
        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn {}", cmd.get_program().display()))?;
        world.app.elapsed = Some(started.elapsed());
        world.note_app_output(&output);
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
    let elapsed = world.app.elapsed.context("the app did not run")?;
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
    let status = world.app.exit_status.context("the app did not run")?;
    if status.code() == Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "the app exited with a startup failure; stderr: {}",
            world.app.stderr
        );
    }
    if !status.success() {
        bail!(
            "the app reported a failure ({status}); stderr: {}",
            world.app.stderr
        );
    }
    if !world.app.stdout.is_empty() {
        bail!("expected no text, got: {:?}", world.app.stdout);
    }
    Ok(())
}

pub fn expect_startup_failure(world: &E2eWorld) -> Result<()> {
    let status = world.app.exit_status.context("the app did not run")?;
    if status.code() != Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "expected a startup failure, got {status}; stderr: {}",
            world.app.stderr
        );
    }
    Ok(())
}

pub fn expect_message(world: &E2eWorld, fragment: &str) -> Result<()> {
    if !world.app.stderr.contains(fragment) {
        bail!(
            "expected '{fragment}' in the app's message, got: {}",
            world.app.stderr
        );
    }
    Ok(())
}
