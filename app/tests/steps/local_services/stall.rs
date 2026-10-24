//! The command spotting client on a fake command spotting service that says, when a step
//! tells it to, that a confirmation stalled.

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};
use tungstenite::Message;

use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::VoiceCommand;

use super::local;
use crate::IntegrationWorld;

const STALLED: &str = r#"{"event":"stalled","waited_ms":12000}"#;
const READ_WAIT: Duration = Duration::from_millis(10);
/// The client reports the loss and is back on a new connection well within this.
const BACK_WITHIN: Duration = Duration::from_secs(5);
const MIC_RATE_HZ: u32 = 48_000;
/// A tenth of a second of silence at the microphone's rate.
const SILENCE: usize = 4_800;

/// The client of one scenario and the fake service it listens through.
pub(super) struct StallRun {
    stream: Option<AudioStream>,
    audio: Sender<Vec<f32>>,
    inputs: Receiver<PipelineInput>,
    /// Set by a step: the service says it stalled on the connection it serves.
    stall: Arc<AtomicBool>,
    accepted: Arc<AtomicUsize>,
    got: Vec<PipelineInput>,
}

impl std::fmt::Debug for StallRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StallRun")
            .field("got", &self.got)
            .finish_non_exhaustive()
    }
}

impl Drop for StallRun {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.stop();
        }
    }
}

/// A fake command spotting service: it keeps every connection and says it stalled once
/// `stall` is set.
fn fake_service(stall: &Arc<AtomicBool>, accepted: &Arc<AtomicUsize>) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("ws://{}", listener.local_addr()?);
    let (stall, accepted) = (Arc::clone(stall), Arc::clone(accepted));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            accepted.fetch_add(1, Ordering::SeqCst);
            let stall = Arc::clone(&stall);
            std::thread::spawn(move || serve(stream, &stall));
        }
    });
    Ok(url)
}

/// Reads until the client leaves, saying it stalled when told to.
fn serve(stream: TcpStream, stall: &AtomicBool) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    if socket.get_ref().set_read_timeout(Some(READ_WAIT)).is_err() {
        return;
    }
    while socket.read().is_ok() || socket.can_read() {
        if stall.swap(false, Ordering::SeqCst) && socket.send(Message::text(STALLED)).is_err() {
            return;
        }
    }
}

#[given("the app listens through a command spotting service")]
fn app_listens(world: &mut IntegrationWorld) -> Result<()> {
    let (stall, accepted) = (Arc::default(), Arc::default());
    let url = fake_service(&stall, &accepted)?;
    let (audio, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs) = mpsc::channel();
    let stream = AudioStream::start(
        &url,
        &VoiceCommand::STANDBY,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )?;
    local(world).stall =
        Some(StallRun { stream: Some(stream), audio, inputs, stall, accepted, got: Vec::new() });
    Ok(())
}

#[when("the command spotting service reports that it stalled")]
fn service_stalls(world: &mut IntegrationWorld) -> Result<()> {
    let run = local(world).stall.as_mut().context("no client listens")?;
    run.stall.store(true, Ordering::SeqCst);
    let deadline = Instant::now() + BACK_WITHIN;
    while Instant::now() < deadline && run.got.last() != Some(&PipelineInput::SpottingBack) {
        run.audio.send(vec![0.0; SILENCE])?;
        run.got
            .extend(run.inputs.recv_timeout(Duration::from_millis(100)));
    }
    Ok(())
}

#[then("the stalled service is counted as lost and the app connects to it again")]
fn lost_and_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = local(world).stall.as_ref().context("no client listens")?;
    ensure!(
        run.got.contains(&PipelineInput::SpottingLost),
        "{:?}",
        run.got
    );
    ensure!(
        run.got.last() == Some(&PipelineInput::SpottingBack),
        "{:?}",
        run.got
    );
    let accepted = run.accepted.load(Ordering::SeqCst);
    ensure!(accepted >= 2, "the app connected {accepted} time(s)");
    Ok(())
}
