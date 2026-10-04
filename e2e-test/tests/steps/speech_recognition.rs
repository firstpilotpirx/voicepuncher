use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use crate::steps::file_transcription::{
    env, sut_bin, write_silence_wav, NO_CONFIG, STARTUP_FAILURE_EXIT_CODE,
};
use crate::E2eWorld;

const MAX_STARTUP_FAILURE: Duration = Duration::from_secs(3);
const REMOTE_SERVICE_URL: &str = "http://192.168.1.10:8081/v1";
const START_COMMAND: &str = "just start-model-servers";
const OUTDATED_CONFIG: &str = "[whisper]\nmodel_path = \"/models/ggml-large-v3-turbo.bin\"\n";

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

#[when("the user starts the app")]
fn start_app(world: &mut E2eWorld) -> Result<()> {
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
