//! The stand-ins the hands free and the session scenarios share: what the pipeline was asked
//! to do, noted in one ordered log, and the command spotting adapter's streaming path fed by
//! the scenario's sound.

use voicepuncher::adapters::secondary::commandspotting::websocket::{
    AudioStream, MicrophoneAudio, SpottingService,
};
use voicepuncher::test_doubles::prelude::*;

/// What the pipeline was asked to do, as a scenario sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Listening,
    RecordingStarted,
    Pasted(String),
    Cancelled,
    ReturnPressed,
    Cleared,
    PasteKeyPressed,
    ListeningOffSound,
    ListeningOnSound,
    /// The pipeline was told the command spotting service went away.
    SpottingLost,
    /// The pipeline was told the command spotting service answers again.
    SpottingBack,
    /// The overlay showed that voice commands do not work.
    CommandsUnavailableShown,
    /// The overlay took that sign away.
    CommandsUnavailableHidden,
}

pub(crate) type Events = Arc<Mutex<Vec<Event>>>;

/// Where a spotter's sound comes from: its rate, and when its capture starts, asked when
/// listening starts.
pub(crate) type SoundSource = Box<dyn FnMut() -> (u32, Instant) + Send>;

/// The command spotting adapter's streaming path, fed by the scenario's sound.
pub(crate) struct StreamingSpotter {
    pub(crate) url: String,
    pub(crate) inputs: Sender<PipelineInput>,
    pub(crate) heard: Option<Receiver<Vec<f32>>>,
    pub(crate) stream: Option<AudioStream>,
    pub(crate) events: Events,
    pub(crate) source: SoundSource,
    /// Whether the pipeline gets what the stream heard, as a lead-in; else an empty buffer.
    pub(crate) keeps_heard: bool,
    /// Whether the service confirms commands before it reports them.
    pub(crate) confirm_commands: bool,
}

impl StreamingSpotter {
    pub(crate) fn new(url: String, inputs: Sender<PipelineInput>, events: &Events) -> Self {
        Self {
            url,
            inputs,
            heard: None,
            stream: None,
            events: Arc::clone(events),
            source: Box::new(|| (AudioBuffer16k::SAMPLE_RATE_HZ, Instant::now())),
            keeps_heard: false,
            confirm_commands: true,
        }
    }
}

impl CommandSpotter for StreamingSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let heard = self.heard.take().context("the sound is already in use")?;
        let (native_rate, captured_from) = (self.source)();
        let audio = MicrophoneAudio { audio: heard, native_rate, captured_from };
        let service = SpottingService {
            confirm_commands: self.confirm_commands,
            ..SpottingService::from(&self.url)
        };
        let stream = AudioStream::start(service, commands, audio, self.inputs.clone())?;
        self.stream = Some(stream);
        self.events.lock().unwrap().push(Event::Listening);
        Ok(())
    }

    fn listen_for(&mut self, commands: &[VoiceCommand]) {
        if let Some(stream) = self.stream.as_ref() {
            stream.listen_for(commands);
        }
    }

    fn stop_listening(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.stop();
        }
    }

    fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        match self.stream.as_ref() {
            Some(stream) if self.keeps_heard => stream.heard_since(since),
            _ => AudioBuffer16k::new(Vec::new()),
        }
    }
}

pub(crate) type RecordingPaster = voicepuncher::test_doubles::NotingPaster<Event>;

pub(crate) struct RecordingKeys(pub(crate) Events);

impl KeyPresser for RecordingKeys {
    fn press_enter(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::ReturnPressed);
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::Cleared);
        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::PasteKeyPressed);
        Ok(())
    }
}

/// Runs the pipeline of `parts` with voice control by `spotter` and keys that note their
/// presses, on a thread of its own, until `inputs` ends.
pub(crate) fn run_with_voice_control(
    parts: PipelineParts,
    spotter: StreamingSpotter,
    events: &Events,
    inputs: Receiver<PipelineInput>,
) {
    let keys = RecordingKeys(Arc::clone(events));
    let mut pipeline = Pipeline::new(parts, PipelineConfig::default())
        .with_voice_control(Box::new(spotter), Box::new(keys));
    std::thread::spawn(move || pipeline.run(inputs));
}
