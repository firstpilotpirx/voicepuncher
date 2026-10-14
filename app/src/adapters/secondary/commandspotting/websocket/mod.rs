//! `CommandSpotter` over the command spotting service's loopback WebSocket: while listening,
//! the microphone streams to the service on a worker thread, and the service's events go
//! into the pipeline's input channel.

mod capture;
pub mod client;
mod heard;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::Result;

use crate::core::application::{CommandSpotter, PipelineInput};
use crate::core::domain::{AudioBuffer16k, VoiceCommand};

use capture::{MicCapture, StreamResampler};
use client::SpottingClient;
use heard::HeardAudio;

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
    /// resampled to 16 kHz; the service's events go to `inputs`.
    pub fn start(
        url: &str,
        commands: &[VoiceCommand],
        audio_start: Instant,
        native_rate: u32,
        audio: Receiver<Vec<f32>>,
        inputs: Sender<PipelineInput>,
    ) -> Result<Self> {
        let mut client = SpottingClient::connect(url, audio_start)?;
        client.listen_for(commands)?;
        let resampler = StreamResampler::new(native_rate)?;
        let (control, control_rx) = mpsc::channel();
        let heard = Arc::new(Mutex::new(HeardAudio::new(audio_start)));
        let streamed = Stream {
            client,
            resampler,
            audio,
            control: control_rx,
            inputs,
            heard: Arc::clone(&heard),
        };
        let worker = std::thread::Builder::new()
            .name("command-spotting".into())
            .spawn(move || stream_to_service(streamed))?;
        Ok(Self {
            control,
            worker,
            heard,
        })
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
    _capture: MicCapture,
}

pub struct WebSocketCommandSpotter {
    url: String,
    inputs: Sender<PipelineInput>,
    listening: Option<Listening>,
}

impl WebSocketCommandSpotter {
    pub fn new(url: String, inputs: Sender<PipelineInput>) -> Self {
        Self {
            url,
            inputs,
            listening: None,
        }
    }
}

impl CommandSpotter for WebSocketCommandSpotter {
    fn probe(&self) -> Result<()> {
        client::probe(&self.url)
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        self.stop_listening();
        let (audio_tx, audio_rx) = mpsc::channel();
        let (capture, native_rate) = MicCapture::open(audio_tx)?;
        log::info!(
            "command spotting: microphone open at {native_rate} Hz, connecting to {}",
            self.url
        );
        let stream = AudioStream::start(
            &self.url,
            commands,
            capture.started_at,
            native_rate,
            audio_rx,
            self.inputs.clone(),
        )?;
        log::info!("command spotting: streaming to the service");
        self.listening = Some(Listening {
            stream,
            _capture: capture,
        });
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

/// What the streaming worker owns.
struct Stream {
    client: SpottingClient,
    resampler: StreamResampler,
    audio: Receiver<Vec<f32>>,
    control: Receiver<Control>,
    inputs: Sender<PipelineInput>,
    heard: Arc<Mutex<HeardAudio>>,
}

/// Runs until told to stop or the service goes away; a lost service is reported once.
fn stream_to_service(stream: Stream) {
    let Stream {
        mut client,
        mut resampler,
        audio,
        control,
        inputs,
        heard,
    } = stream;
    loop {
        match control.try_recv() {
            Ok(Control::ListenFor(commands)) => {
                if let Err(e) = client.listen_for(&commands) {
                    return report_lost(&inputs, e);
                }
            }
            Ok(Control::Stop) | Err(TryRecvError::Disconnected) => return client.close(),
            Err(TryRecvError::Empty) => {}
        }
        let captured: Vec<f32> = audio.try_iter().flatten().collect();
        let samples = resampler.push(&captured);
        if !samples.is_empty() {
            if let Ok(mut heard) = heard.lock() {
                heard.push(&samples);
            }
            if let Err(e) = client.send_audio(&samples) {
                return report_lost(&inputs, e);
            }
        }
        if let Err(e) = client.keep_alive() {
            return report_lost(&inputs, e);
        }
        match client.read_events() {
            Ok(events) => {
                for event in events {
                    let _ = inputs.send(PipelineInput::Spotting(event));
                }
            }
            Err(e) => return report_lost(&inputs, e),
        }
    }
}

fn report_lost(inputs: &Sender<PipelineInput>, error: anyhow::Error) {
    log::error!("{error:#}");
    let _ = inputs.send(PipelineInput::SpottingLost);
}
