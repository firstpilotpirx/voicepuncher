//! Another program, no speech recognition service, on the speech recognition service's own
//! port, as the client of the service logs it.

use cucumber::{given, then, when};
use std::net::TcpListener;
use voicepuncher::test_doubles::prelude::*;
use voicepuncher::{
    adapters::secondary::transcription::openai::OpenAiTranscriber,
    config::TranscriptionCfg,
    core::{application::Transcriber, domain::AudioBuffer16k},
};
use voicepuncher_test_steps::loopback::Http2Only;

use super::fake_copies::{free_copies, serve, FakeCopy};
use super::{local, lock};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// What the scenario set up: the service's address, the other program on its own port and
/// the one copy that answers.
#[derive(Debug)]
pub(super) struct ForeignRun {
    base_url: String,
    port: u16,
    _program: Http2Only,
    _copy: FakeCopy,
}

fn foreign(world: &mut IntegrationWorld) -> Result<&mut ForeignRun> {
    local(world)
        .foreign
        .as_mut()
        .context("no other program was started")
}

#[given("another program that is no speech recognition service listens on the own port of the speech recognition service, which runs as one copy")]
fn program_on_service_port(world: &mut IntegrationWorld) -> Result<()> {
    pipeline_log();
    let (base_url, mut listeners) = free_copies(1)?;
    let port = base_url
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/v1")
        .parse()?;
    let program = Http2Only::on(TcpListener::bind(("127.0.0.1", port))?);
    let copy = serve(listeners.remove(0));
    local(world).foreign = Some(ForeignRun { base_url, port, _program: program, _copy: copy });
    Ok(())
}

#[when("a client of the service transcribes two dictations")]
fn transcribes_twice(world: &mut IntegrationWorld) -> Result<()> {
    let run = foreign(world)?;
    let cfg = TranscriptionCfg { base_url: run.base_url.clone(), ..TranscriptionCfg::default() };
    let mut client = OpenAiTranscriber::connect(cfg)?;
    for _ in 0..2 {
        client.transcribe(&AudioBuffer16k::new(vec![100i16; 8_000]))?;
    }
    Ok(())
}

#[then("the log holds one warning naming that port, the program's name and its process id")]
fn logged_once(world: &mut IntegrationWorld) -> Result<()> {
    let port = foreign(world)?.port;
    let held = format!("port {port} of the service at http://127.0.0.1:{port}/v1 is held by ");
    let warnings: Vec<String> = lock(&pipeline_log().warnings)
        .iter()
        .map(|(_, line)| line.clone())
        .filter(|line| line.starts_with(&held))
        .collect();
    ensure!(
        warnings.len() == 1,
        "expected one warning {held:?}…, got {warnings:?}"
    );
    let exe = std::env::current_exe()?;
    let name: String = exe
        .file_name()
        .context("no name")?
        .to_string_lossy()
        .chars()
        .take(9)
        .collect();
    let pid = format!(" (pid {})", std::process::id());
    ensure!(
        warnings[0].contains(&name) && warnings[0].contains(&pid),
        "the warning does not name {name:?} and{pid}: {}",
        warnings[0]
    );
    Ok(())
}
