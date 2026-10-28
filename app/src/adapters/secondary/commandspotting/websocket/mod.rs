//! `CommandSpotter` over the command spotting service's loopback WebSocket.
//!
//! While listening, the microphone streams to the service on a worker thread, and the service's
//! events go into the pipeline's input channel.

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
use crate::adapters::secondary::recording::cpal::tap::MicTap;
use crate::adapters::secondary::recording::cpal::{devices, resampler::StreamResampler};
use crate::core::application::{CommandSpotter, PipelineInput};
use crate::core::domain::{AudioBuffer16k, SpottingEvent, VoiceCommand};

use client::SpottingClient;
use heard::HeardAudio;
use kept_mic::{KeeperTimings, KeptMic, MicOpener};
pub use link::LinkTimings;
use link::Outage;

mod stream;
pub mod test_stream;
use stream::{stream_to_service, Stream};

/// How long the worker waits between rounds while the service is away.
const AWAY_WAIT: Duration = Duration::from_millis(10);

/// Numbers each connection to the service, for the log: `session=<n>`.
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

enum Control {
    ListenFor(Vec<VoiceCommand>),
    Stop,
}

/// The audio a microphone captures: its samples, their rate and when the first was captured.
pub struct MicrophoneAudio<A> {
    pub audio: A,
    pub native_rate: u32,
    pub captured_from: Instant,
}

/// The command spotting service to stream to: its address, whether it confirms commands
/// before it reports them, and who hears of a copy that refused the app's protocol version.
#[derive(Debug, Clone)]
pub struct SpottingService {
    pub url: String,
    pub confirm_commands: bool,
    pub refused: RefusedCopies,
}

impl From<&str> for SpottingService {
    fn from(url: &str) -> Self {
        Self { url: url.to_owned(), confirm_commands: true, refused: RefusedCopies::default() }
    }
}

/// Told the port of every copy that refuses the app's protocol version; by default no one.
#[derive(Clone, Default)]
pub struct RefusedCopies(Option<Arc<dyn Fn(u16) + Send + Sync>>);

impl RefusedCopies {
    pub fn new(tell: impl Fn(u16) + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(tell)))
    }

    pub fn report(&self, port: u16) {
        if let Some(tell) = &self.0 {
            tell(port);
        }
    }
}

impl std::fmt::Debug for RefusedCopies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "RefusedCopies(told)"
        } else {
            "RefusedCopies(none)"
        })
    }
}

impl From<&String> for SpottingService {
    fn from(url: &String) -> Self {
        Self::from(url.as_str())
    }
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
    pub fn start(
        service: impl Into<SpottingService>,
        commands: &[VoiceCommand],
        microphone: MicrophoneAudio<impl CapturedAudio>,
        inputs: Sender<PipelineInput>,
    ) -> Result<Self> {
        Self::start_with_timings(
            service,
            commands,
            microphone,
            inputs,
            LinkTimings::default(),
        )
    }

    /// As `start`, pacing the attempts after a loss and the replay by `timings`.
    pub fn start_with_timings(
        service: impl Into<SpottingService>,
        commands: &[VoiceCommand],
        microphone: MicrophoneAudio<impl CapturedAudio>,
        inputs: Sender<PipelineInput>,
        timings: LinkTimings,
    ) -> Result<Self> {
        let MicrophoneAudio { audio, native_rate, captured_from: audio_start } = microphone;
        let service = service.into();
        let refused = |port| service.refused.report(port);
        let mut client = SpottingClient::connect_reporting(&service.url, audio_start, &refused)?
            .confirming(service.confirm_commands);
        client.listen_for(commands)?;
        let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        log::info!(
            "command spotting: session={session} connected to {}",
            service.url
        );
        let resampler = StreamResampler::new(native_rate)?;
        let (control, control_rx) = mpsc::channel();
        let heard = Arc::new(Mutex::new(HeardAudio::new(audio_start)));
        let streamed = Stream {
            service,
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
    /// Whether the service confirms commands before it reports them.
    confirm_commands: bool,
    inputs: Sender<PipelineInput>,
    listening: Option<Listening>,
    /// The configuration's `input_device`: a device name or `default`.
    input_device: String,
    /// Where the kept microphone's audio is shared with the dictation recorder.
    tap: MicTap,
    refused: RefusedCopies,
}

impl WebSocketCommandSpotter {
    pub fn new(url: String, inputs: Sender<PipelineInput>) -> Self {
        Self {
            url,
            confirm_commands: true,
            inputs,
            listening: None,
            input_device: devices::DEFAULT_DEVICE.to_owned(),
            tap: MicTap::default(),
            refused: RefusedCopies::default(),
        }
    }

    /// Tells `refused` the port of every copy that refuses the app's protocol version.
    pub fn reporting_refused(mut self, refused: RefusedCopies) -> Self {
        self.refused = refused;
        self
    }

    /// Shares the kept microphone's audio through `tap` while listening.
    pub fn with_tap(mut self, tap: MicTap) -> Self {
        self.tap = tap;
        self
    }

    /// Has the service confirm commands before it reports them, or not.
    #[must_use]
    pub const fn confirming(mut self, confirm_commands: bool) -> Self {
        self.confirm_commands = confirm_commands;
        self
    }

    /// Listens on the input device `name` names (see [`devices::open`]).
    pub fn with_input_device(mut self, name: &str) -> Self {
        name.clone_into(&mut self.input_device);
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
        let service = SpottingService {
            url: self.url.clone(),
            confirm_commands: self.confirm_commands,
            refused: self.refused.clone(),
        };
        let stream = AudioStream::start(
            service,
            commands,
            MicrophoneAudio {
                audio: self.tap.tee(Box::new(audio)),
                native_rate: AudioBuffer16k::SAMPLE_RATE_HZ,
                captured_from: started_at,
            },
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
