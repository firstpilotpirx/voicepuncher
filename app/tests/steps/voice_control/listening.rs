//! Listening kept alive: the command spotting microphone opened again when it stalls or
//! fails (`KeptMic` over stand-in microphones the scenario feeds), and the command spotting
//! link reached again after the service dropped audio (`AudioStream` against a stand-in
//! service on loopback).

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicU32;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use rtrb::{Producer, RingBuffer};
use tungstenite::Message;

use voicepuncher::adapters::secondary::commandspotting::websocket::kept_mic::{
    KeeperTimings, KeptMic, MicOpener, OpenMic,
};
use voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::take_greeting;
use voicepuncher::adapters::secondary::commandspotting::websocket::{AudioStream, MicrophoneAudio};
use voicepuncher::adapters::secondary::recording::cpal::capture::{on_frames, CaptureHealth};

use super::pipeline_log;
use crate::IntegrationWorld;

const STALLED: &str = "command spotting: microphone delivered no audio";
const CANNOT_OPEN: &str = "command spotting: cannot open the microphone again";
/// The first audio, and the audio of the reopened microphone, as the scenario tells them apart.
const FIRST_LEVEL: f32 = 0.5;
const REOPENED_LEVEL: f32 = 0.25;
/// How long a step waits for the keeper or the link to act.
const WAIT: Duration = Duration::from_secs(6);
/// The spec's bound for a stalled microphone to be opened again.
const REOPENED_WITHIN: Duration = Duration::from_secs(3);

/// A stand-in microphone the scenario feeds.
struct StandInMic {
    ring: Producer<f32>,
    health: Arc<CaptureHealth>,
    started_at: Instant,
}

/// What a listening scenario keeps between its steps.
#[derive(Default)]
pub struct ListeningRun {
    /// Kept only so the microphone keeps listening until the scenario ends.
    _keeper: Option<KeptMic>,
    audio: Option<Receiver<Vec<f32>>>,
    started_at: Option<Instant>,
    mics: Option<Receiver<StandInMic>>,
    opens: Arc<AtomicUsize>,
    /// The microphone open now.
    mic: Option<StandInMic>,
    reopened_after: Option<Duration>,
    warnings: Vec<String>,
    inputs: Vec<PipelineInput>,
    connections: Arc<AtomicUsize>,
    url: Option<String>,
}

impl std::fmt::Debug for ListeningRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListeningRun")
            .field("opens", &self.opens)
            .field("inputs", &self.inputs)
            .finish_non_exhaustive()
    }
}

fn listening(world: &mut IntegrationWorld) -> Result<&mut ListeningRun> {
    world
        .extra
        .speech
        .listening
        .as_mut()
        .context("no listening scenario")
}

/// Opens a stand-in microphone per call, failing the calls `failing` names, and hands each
/// one it opened to the scenario.
fn stand_in_opener(
    failing: &'static [usize],
    opens: Arc<AtomicUsize>,
) -> (MicOpener, Receiver<StandInMic>) {
    let (handed, mics) = mpsc::channel();
    let open = move || -> Result<OpenMic> {
        if failing.contains(&opens.fetch_add(1, Ordering::SeqCst)) {
            return Err(anyhow!("the input device is busy"));
        }
        let (ring, audio) = RingBuffer::new(AudioBuffer16k::SAMPLE_RATE_HZ as usize);
        let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
        let mic = StandInMic { ring, health: Arc::clone(&health), started_at: Instant::now() };
        let started_at = mic.started_at;
        let _ = handed.send(mic);
        let rate = AudioBuffer16k::SAMPLE_RATE_HZ;
        Ok(OpenMic { audio: Box::new(audio), health, rate, started_at, device: Box::new(()) })
    };
    (Box::new(open), mics)
}

/// Starts listening on stand-in microphones and lets the first deliver audio.
fn listen(
    world: &mut IntegrationWorld,
    failing: &'static [usize],
    timings: KeeperTimings,
) -> Result<()> {
    pipeline_log();
    let opens = Arc::new(AtomicUsize::new(0));
    let (open, mics) = stand_in_opener(failing, Arc::clone(&opens));
    let (keeper, audio, started_at) = KeptMic::start(open, timings)?;
    let mut first = mics.recv_timeout(WAIT)?;
    on_frames(&[FIRST_LEVEL; 160], 1, &mut first.ring, &first.health);
    world.extra.speech.listening = Some(ListeningRun {
        _keeper: Some(keeper),
        audio: Some(audio),
        started_at: Some(started_at),
        mics: Some(mics),
        opens,
        mic: Some(first),
        ..ListeningRun::default()
    });
    Ok(())
}

#[given("listening is on and the microphone delivered audio")]
#[given("the microphone was silent until it was opened again")]
fn delivered(world: &mut IntegrationWorld) -> Result<()> {
    listen(world, &[], KeeperTimings::default())
}

#[given("listening is on with a microphone that is never judged stalled")]
fn never_stalls(world: &mut IntegrationWorld) -> Result<()> {
    let timings = KeeperTimings { stall: Duration::from_secs(600), ..KeeperTimings::default() };
    listen(world, &[], timings)
}

#[given("the microphone stalled, and opening it fails twice and then succeeds")]
fn reopen_fails_twice(world: &mut IntegrationWorld) -> Result<()> {
    listen(world, &[1, 2], KeeperTimings::default())
}

/// Waits for the microphone to be opened again and lets it deliver audio of its own.
fn reopened(world: &mut IntegrationWorld) -> Result<()> {
    let before = pipeline_log().warnings.lock().unwrap().len();
    let run = listening(world)?;
    let waited = Instant::now();
    let mics = run.mics.as_ref().context("no microphone")?;
    let mut mic = mics
        .recv_timeout(WAIT + WAIT)
        .context("the microphone was not opened again")?;
    run.reopened_after = Some(waited.elapsed());
    on_frames(&[REOPENED_LEVEL; 16], 1, &mut mic.ring, &mic.health);
    run.mic = Some(mic);
    run.warnings = pipeline_log().warnings.lock().unwrap()[before..]
        .iter()
        .map(|(_, line)| line.clone())
        .collect();
    Ok(())
}

#[when("the microphone stops delivering audio without reporting an error")]
#[when("the app tries to open the microphone again")]
#[when("the reopened microphone delivers audio")]
fn stops_delivering(world: &mut IntegrationWorld) -> Result<()> {
    reopened(world)
}

#[when("the microphone reports an error")]
fn reports_error(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    run.mic
        .as_ref()
        .context("no microphone")?
        .health
        .note_error();
    reopened(world)
}

/// Every sample listening sent until the reopened microphone's audio, none when it never came.
fn streamed(run: &ListeningRun) -> Result<Vec<f32>> {
    let audio = run.audio.as_ref().context("listening is not on")?;
    let mut taken = Vec::new();
    while !taken
        .iter()
        .any(|&s: &f32| (s - REOPENED_LEVEL).abs() < f32::EPSILON)
    {
        taken.extend(
            audio
                .recv_timeout(WAIT)
                .context("the reopened audio never came")?,
        );
    }
    Ok(taken)
}

fn logged(run: &ListeningRun, line: &str) -> usize {
    run.warnings.iter().filter(|w| w.starts_with(line)).count()
}

#[then("within 3 seconds the app logs that the microphone stalled and opens it again")]
fn stall_logged(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    let after = run.reopened_after.context("not opened again")?;
    if after > REOPENED_WITHIN || logged(run, STALLED) != 1 {
        bail!("opened again after {after:?}, warnings {:?}", run.warnings);
    }
    Ok(())
}

#[then("the audio of the reopened microphone reaches the command spotting service")]
fn reopened_audio_streamed(world: &mut IntegrationWorld) -> Result<()> {
    let taken = streamed(listening(world)?)?;
    if !taken
        .iter()
        .any(|&s| (s - FIRST_LEVEL).abs() < f32::EPSILON)
    {
        bail!("the first microphone's audio is missing from the stream");
    }
    Ok(())
}

#[then("the app opens it again")]
fn opened_again(world: &mut IntegrationWorld) -> Result<()> {
    let opens = listening(world)?.opens.load(Ordering::SeqCst);
    if opens != 2 {
        bail!("the microphone was opened {opens} times instead of 2");
    }
    Ok(())
}

#[then("the app keeps trying every second until it is open, logging the failure once")]
fn kept_trying(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    let opens = run.opens.load(Ordering::SeqCst);
    if opens != 4 || logged(run, CANNOT_OPEN) != 1 {
        bail!("{opens} opens, warnings {:?}", run.warnings);
    }
    Ok(())
}

#[then("that audio is streamed after silence, so its time in the stream is its capture time")]
fn at_capture_time(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    let taken = streamed(run)?;
    let at = taken
        .iter()
        .position(|&s| (s - REOPENED_LEVEL).abs() < f32::EPSILON)
        .context("no reopened audio")?;
    let start = run.started_at.context("listening is not on")?;
    let captured = run
        .mic
        .as_ref()
        .context("no microphone")?
        .started_at
        .duration_since(start);
    let due = captured.as_secs_f64() * f64::from(AudioBuffer16k::SAMPLE_RATE_HZ);
    let silent = taken
        .get(160..at)
        .unwrap_or_default()
        .iter()
        .all(|&s| s.abs() < f32::EPSILON);
    if (f64::from(u32::try_from(at).unwrap_or(u32::MAX)) - due).abs() > 16.0 || !silent {
        bail!("the reopened audio sits at sample {at}, captured at sample {due:.0}, silent between: {silent}");
    }
    Ok(())
}

/// Accepts every connection and sends `{"event":"dropped"}` on the first only.
fn dropping_service(connections: Arc<AtomicUsize>) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("ws://{}", listener.local_addr()?);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let first = connections.fetch_add(1, Ordering::SeqCst) == 0;
            std::thread::spawn(move || hold_connection(stream, first));
        }
    });
    Ok(url)
}

fn hold_connection(stream: TcpStream, report_drop: bool) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    let _ = socket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(10)));
    take_greeting(&mut socket);
    if report_drop {
        let _ = socket.send(Message::text(r#"{"event":"dropped","samples":1600}"#));
    }
    while socket.read().is_ok() || socket.can_read() {}
}

#[given("listening is on with a command spotting service that will drop audio")]
fn service_will_drop(world: &mut IntegrationWorld) -> Result<()> {
    let connections = Arc::new(AtomicUsize::new(0));
    let url = dropping_service(Arc::clone(&connections))?;
    world.extra.speech.listening =
        Some(ListeningRun { url: Some(url), connections, ..ListeningRun::default() });
    Ok(())
}

#[when("the command spotting service reports dropped audio")]
fn reports_dropped(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    let url = run.url.clone().context("no service")?;
    let (mic, heard) = mpsc::channel();
    let (inputs_tx, inputs) = mpsc::channel();
    let stream = AudioStream::start(
        &url,
        &VoiceCommand::STANDBY,
        MicrophoneAudio { audio: heard, native_rate: 48_000, captured_from: Instant::now() },
        inputs_tx,
    )?;
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline && run.inputs.last() != Some(&PipelineInput::SpottingBack) {
        mic.send(vec![0.0; 4_800])?;
        run.inputs
            .extend(inputs.recv_timeout(Duration::from_millis(100)));
    }
    stream.stop();
    Ok(())
}

#[then("the service is counted as lost and the app reaches for it again")]
fn reached_again(world: &mut IntegrationWorld) -> Result<()> {
    let run = listening(world)?;
    let lost = run.inputs.contains(&PipelineInput::SpottingLost);
    let back = run.inputs.last() == Some(&PipelineInput::SpottingBack);
    if !lost || !back || run.connections.load(Ordering::SeqCst) < 2 {
        bail!(
            "inputs {:?} over {} connections",
            run.inputs,
            run.connections.load(Ordering::SeqCst)
        );
    }
    Ok(())
}
