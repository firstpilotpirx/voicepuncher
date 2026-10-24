//! `CommandSpotter` over the command spotting service's loopback WebSocket: while listening,
//! the microphone streams to the service on a worker thread, and the service's events go
//! into the pipeline's input channel.

pub mod client;
mod heard;
pub mod kept_mic;
mod link;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::adapters::secondary::recording::cpal::capture::CapturedAudio;
use crate::adapters::secondary::recording::cpal::{devices, resampler::StreamResampler};
use crate::core::application::{CommandSpotter, PipelineInput};
use crate::core::domain::{AudioBuffer16k, SpottingEvent, VoiceCommand};

use client::SpottingClient;
use heard::HeardAudio;
use kept_mic::{KeeperTimings, KeptMic, MicOpener};
pub use link::LinkTimings;
use link::Outage;

/// How long the worker waits between rounds while the service is away.
const AWAY_WAIT: Duration = Duration::from_millis(10);

/// Numbers each connection to the service, for the log: `session=<n>`.
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

enum Control {
    ListenFor(Vec<VoiceCommand>),
    Stop,
}

/// Audio streaming to the service on a worker thread, whatever captures it.
pub struct AudioStream {
    control: Sender<Control>,
    worker: JoinHandle<()>,
    heard: Arc<Mutex<HeardAudio>>,
}

impl AudioStream {
    /// Connects to the service at `url`, listens for `commands`, and streams every chunk
    /// from `audio` — mono at `native_rate`, the first sample captured at `audio_start` —
    /// resampled to 16 kHz; the service's events go to `inputs`. When the service goes
    /// away the worker reaches for it again until stopped, and tells `inputs` of the loss
    /// and of the return.
    #[expect(clippy::too_many_arguments, reason = "size baseline")]
    pub fn start(
        url: &str,
        commands: &[VoiceCommand],
        audio_start: Instant,
        native_rate: u32,
        audio: impl CapturedAudio,
        inputs: Sender<PipelineInput>,
    ) -> Result<Self> {
        Self::start_with_timings(
            url,
            commands,
            audio_start,
            native_rate,
            audio,
            inputs,
            LinkTimings::default(),
        )
    }

    /// As `start`, pacing the attempts after a loss and the replay by `timings`.
    #[expect(clippy::too_many_arguments, reason = "size baseline")]
    pub fn start_with_timings(
        url: &str,
        commands: &[VoiceCommand],
        audio_start: Instant,
        native_rate: u32,
        audio: impl CapturedAudio,
        inputs: Sender<PipelineInput>,
        timings: LinkTimings,
    ) -> Result<Self> {
        let mut client = SpottingClient::connect(url, audio_start)?;
        client.listen_for(commands)?;
        let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        log::info!("command spotting: session={session} connected to {url}");
        let resampler = StreamResampler::new(native_rate)?;
        let (control, control_rx) = mpsc::channel();
        let heard = Arc::new(Mutex::new(HeardAudio::new(audio_start)));
        let streamed = Stream {
            url: url.to_owned(),
            session,
            link: Link::Up(Box::new(client)),
            timings,
            commands: commands.to_vec(),
            last_command_end: None,
            resampler,
            audio: Box::new(audio),
            control: control_rx,
            inputs,
            heard: Arc::clone(&heard),
        };
        let worker = std::thread::Builder::new()
            .name("command-spotting".into())
            .spawn(move || stream_to_service(streamed))?;
        Ok(Self { control, worker, heard })
    }

    /// The audio streamed from `since` on, as far as it is still held.
    pub fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        self.heard
            .lock()
            .map_or_else(|_| AudioBuffer16k::new(Vec::new()), |h| h.since(since))
    }

    pub fn listen_for(&self, commands: &[VoiceCommand]) {
        let _ = self.control.send(Control::ListenFor(commands.to_vec()));
    }

    pub fn stop(self) {
        let _ = self.control.send(Control::Stop);
        if self.worker.join().is_err() {
            log::warn!("command spotting worker panicked");
        }
    }
}

struct Listening {
    stream: AudioStream,
    _mic: KeptMic,
}

pub struct WebSocketCommandSpotter {
    url: String,
    inputs: Sender<PipelineInput>,
    listening: Option<Listening>,
    /// The configuration's `input_device`: a device name or `default`.
    input_device: String,
}

impl WebSocketCommandSpotter {
    pub fn new(url: String, inputs: Sender<PipelineInput>) -> Self {
        Self { url, inputs, listening: None, input_device: devices::DEFAULT_DEVICE.to_owned() }
    }

    /// Listens on the input device `name` names (see [`devices::open`]).
    pub fn with_input_device(mut self, name: &str) -> Self {
        self.input_device = name.to_owned();
        self
    }
}

impl CommandSpotter for WebSocketCommandSpotter {
    fn probe(&self) -> Result<()> {
        client::probe(&self.url)
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        self.stop_listening();
        let device = self.input_device.clone();
        let open: MicOpener = Box::new(move || kept_mic::open_device(&device));
        let (mic, audio, started_at) = KeptMic::start(open, KeeperTimings::default())?;
        log::info!("command spotting: connecting to {}", self.url);
        let stream = AudioStream::start(
            &self.url,
            commands,
            started_at,
            AudioBuffer16k::SAMPLE_RATE_HZ,
            audio,
            self.inputs.clone(),
        )?;
        log::info!("command spotting: streaming to the service");
        self.listening = Some(Listening { stream, _mic: mic });
        Ok(())
    }

    fn listen_for(&mut self, commands: &[VoiceCommand]) {
        if let Some(listening) = self.listening.as_ref() {
            listening.stream.listen_for(commands);
        }
    }

    fn stop_listening(&mut self) {
        if let Some(listening) = self.listening.take() {
            listening.stream.stop();
        }
    }

    fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        self.listening.as_ref().map_or_else(
            || AudioBuffer16k::new(Vec::new()),
            |listening| listening.stream.heard_since(since),
        )
    }
}

/// The connection to the service, or the outage while it is away.
enum Link {
    Up(Box<SpottingClient>),
    Away(Outage),
}

/// What the streaming worker owns.
struct Stream {
    url: String,
    session: u64,
    link: Link,
    /// How the attempts after a loss are paced and how far back the replay reaches.
    timings: LinkTimings,
    /// The commands listened for now, sent again to a new connection.
    commands: Vec<VoiceCommand>,
    /// The end of the last command reported: a replay never reaches back to it.
    last_command_end: Option<Instant>,
    resampler: StreamResampler,
    audio: Box<dyn CapturedAudio>,
    control: Receiver<Control>,
    inputs: Sender<PipelineInput>,
    heard: Arc<Mutex<HeardAudio>>,
}

/// Runs until told to stop; a lost service is reported once, a long outage once, and its
/// return once.
fn stream_to_service(mut stream: Stream) {
    loop {
        match stream.control.try_recv() {
            Ok(Control::ListenFor(commands)) => stream.listen_for(commands),
            Ok(Control::Stop) | Err(TryRecvError::Disconnected) => return stream.close(),
            Err(TryRecvError::Empty) => {}
        }
        stream.stream_audio();
        match &stream.link {
            Link::Up(_) => stream.exchange(),
            Link::Away(_) => stream.reach_for_service(),
        }
    }
}

impl Stream {
    fn listen_for(&mut self, commands: Vec<VoiceCommand>) {
        log::info!(
            "command spotting: session={} listening for {commands:?}",
            self.session
        );
        self.commands = commands;
        if let Link::Up(client) = &mut self.link {
            if let Err(e) = client.listen_for(&self.commands) {
                self.lose(e);
            }
        }
    }

    /// Holds every sample captured, and sends it while the service is there.
    fn stream_audio(&mut self) {
        let captured = self.audio.take();
        let samples = self.resampler.push(&captured);
        if samples.is_empty() {
            return;
        }
        if let Ok(mut heard) = self.heard.lock() {
            heard.push(&samples, Instant::now());
        }
        if let Link::Up(client) = &mut self.link {
            if let Err(e) = client.send_audio(&samples) {
                self.lose(e);
            }
        }
    }

    /// Checks that the service answers and passes its events on.
    fn exchange(&mut self) {
        let Link::Up(client) = &mut self.link else {
            return;
        };
        let events = client.keep_alive().and_then(|()| client.read_events());
        match events {
            Ok(events) => {
                for mut event in events {
                    self.note_command_end(&event);
                    self.to_capture_times(&mut event);
                    let _ = self.inputs.send(PipelineInput::Spotting(event));
                }
            }
            Err(e) => self.lose(e),
        }
    }

    /// The report times of `event`, dated by stream position, as the moments that audio was
    /// captured: they fall behind the clock once audio was lost. Its own times stay, as the
    /// lead-in is taken by them.
    fn to_capture_times(&self, event: &mut SpottingEvent) {
        let (SpottingEvent::PhraseSpotted { timing: Some(report), .. }
        | SpottingEvent::CommandsSaid { timing: Some(report), .. }) = event
        else {
            return;
        };
        let Ok(heard) = self.heard.lock() else {
            return;
        };
        report.words_ended = heard.captured_at(report.words_ended);
        if let Some(confirmation) = report.confirmation.as_mut() {
            confirmation.asked_at = heard.captured_at(confirmation.asked_at);
        }
    }

    fn note_command_end(&mut self, event: &SpottingEvent) {
        let end = match event {
            SpottingEvent::PhraseSpotted { end, .. } | SpottingEvent::CommandsSaid { end, .. } => {
                *end
            }
            _ => return,
        };
        self.last_command_end = Some(self.last_command_end.map_or(end, |last| last.max(end)));
    }

    fn lose(&mut self, error: anyhow::Error) {
        log::error!("command spotting: session={} lost: {error:#}", self.session);
        let lost_at = Instant::now();
        if let Link::Up(client) = std::mem::replace(
            &mut self.link,
            Link::Away(Outage::since(lost_at, self.timings)),
        ) {
            client.close();
        }
        let _ = self.inputs.send(PipelineInput::SpottingLost);
    }

    /// Tries the service again when the pause after the last attempt is over; tells the
    /// pipeline once when the outage has lasted too long for the replay to hide it.
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn reach_for_service(&mut self) {
        let Link::Away(outage) = &mut self.link else {
            return;
        };
        let now = Instant::now();
        if outage.became_long(now) {
            let _ = self.inputs.send(PipelineInput::SpottingStillAway);
        }
        if !outage.due(now) {
            std::thread::sleep(AWAY_WAIT);
            return;
        }
        // The loss dated by stream position, as the command ends and the replay are.
        let lost_at = outage.lost_at;
        let replay_from = self
            .timings
            .replay_start(heard::dated(&self.heard, lost_at), self.last_command_end);
        match self.reconnect(replay_from) {
            Ok((client, replayed)) => {
                self.session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
                log::info!(
                    "command spotting: session={} back after {:.1}s away, {:.1}s of audio sent again",
                    self.session,
                    lost_at.elapsed().as_secs_f32(),
                    replayed.as_secs_f32()
                );
                self.link = Link::Up(Box::new(client));
                let _ = self.inputs.send(PipelineInput::SpottingBack);
            }
            Err(e) => {
                let Link::Away(outage) = &mut self.link else {
                    return;
                };
                let pause = outage.failed(now);
                if link::attempt_logged(outage.failed_attempts) {
                    log::info!(
                        "command spotting: session={} attempt {} to reach {} failed, next in {}ms: {e:#}",
                        self.session,
                        outage.failed_attempts,
                        self.url,
                        pause.as_millis()
                    );
                }
            }
        }
    }

    /// A new connection, told the current commands and sent the audio held from `replay_from`.
    fn reconnect(&self, replay_from: Instant) -> Result<(SpottingClient, Duration)> {
        let (audio_start, samples) = self.heard.lock().map_or_else(
            |_| (Instant::now(), Vec::new()),
            |heard| heard.replay_from(replay_from),
        );
        let mut client = SpottingClient::connect(&self.url, audio_start)?;
        client.listen_for(&self.commands)?;
        if !samples.is_empty() {
            client.send_audio(&samples)?;
        }
        let replayed =
            Duration::from_secs_f32(samples.len() as f32 / AudioBuffer16k::SAMPLE_RATE_HZ as f32);
        Ok((client, replayed))
    }

    fn close(self) {
        if let Link::Up(client) = self.link {
            client.close();
        }
        log::info!("command spotting: session={} closed", self.session);
    }
}
