//! Stand-ins for the ports the pipeline drives in the voice-control scenarios. Each records
//! what it was asked to do as an `Event` in one shared, ordered log.

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;

use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, SoundPlayer, TextPaster, Transcriber,
};
use voicepuncher::core::domain::{AudioBuffer16k, CleanedText, RawTranscript, VoiceCommand};

/// Long enough that a command spoken right after "stop dictation" arrives before the paste.
pub const TRANSCRIPTION_TIME: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    RecordingStarted,
    RecordingCancelled,
    Pasted(String),
    ReturnPressed,
    AllTextCleared,
    Sound(&'static str),
    ListeningStarted,
    ListeningStopped,
}

pub type Events = Arc<Mutex<Vec<Event>>>;

/// What the user said into the running recording, with the moment each part started.
pub type Speech = Arc<Mutex<Vec<(Instant, String)>>>;

pub type Cut = Arc<Mutex<Option<Instant>>>;

pub fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

pub struct NotingRecorder {
    pub events: Events,
    pub speech: Speech,
    pub cut: Cut,
}

impl AudioRecorder for NotingRecorder {
    fn start(&mut self) -> Result<()> {
        self.speech.lock().unwrap().clear();
        *self.cut.lock().unwrap() = None;
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let one_second = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
        Ok(AudioBuffer16k::new(vec![1_000i16; one_second]))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}

    fn truncate_at(&mut self, cut: Instant) {
        *self.cut.lock().unwrap() = Some(cut);
    }
}

/// Answers every part of the speech that started before the recording's cut.
pub struct SpeechBeforeCut {
    pub speech: Speech,
    pub cut: Cut,
}

impl Transcriber for SpeechBeforeCut {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        std::thread::sleep(TRANSCRIPTION_TIME);
        let cut = *self.cut.lock().unwrap();
        let heard: Vec<String> = self
            .speech
            .lock()
            .unwrap()
            .iter()
            .filter(|(at, _)| cut.is_none_or(|cut| *at < cut))
            .map(|(_, words)| words.clone())
            .collect();
        if heard.is_empty() {
            return Ok(None);
        }
        Ok(Some(RawTranscript::new(heard.join(" "))?))
    }
}

pub struct RecordingPaster(pub Events);

impl TextPaster for RecordingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        record(&self.0, Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub struct RecordingKeyPresser(pub Events);

impl KeyPresser for RecordingKeyPresser {
    fn press_enter(&mut self) -> Result<()> {
        record(&self.0, Event::ReturnPressed);
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        record(&self.0, Event::AllTextCleared);
        Ok(())
    }
}

pub struct RecordingSounds(pub Events);

impl SoundPlayer for RecordingSounds {
    fn play_start(&self) {
        record(&self.0, Event::Sound("start"));
    }

    fn play_stop(&self) {
        record(&self.0, Event::Sound("stop"));
    }

    fn play_cancel(&self) {
        record(&self.0, Event::RecordingCancelled);
        record(&self.0, Event::Sound("cancel"));
    }

    fn play_listening_on(&self) {
        record(&self.0, Event::Sound("listening on"));
    }

    fn play_listening_off(&self) {
        record(&self.0, Event::Sound("listening off"));
    }

    fn play_command(&self) {
        record(&self.0, Event::Sound("command"));
    }
}

/// The command spotting service as the pipeline sees it; the scenario feeds its events
/// straight into the pipeline's input channel.
pub struct ScriptedSpotter {
    pub events: Events,
    pub phrases: Arc<Mutex<Vec<VoiceCommand>>>,
}

impl CommandSpotter for ScriptedSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        *self.phrases.lock().unwrap() = commands.to_vec();
        record(&self.events, Event::ListeningStarted);
        Ok(())
    }

    fn listen_for(&mut self, commands: &[VoiceCommand]) {
        *self.phrases.lock().unwrap() = commands.to_vec();
    }

    fn stop_listening(&mut self) {
        self.phrases.lock().unwrap().clear();
        record(&self.events, Event::ListeningStopped);
    }
}

/// Keeps every error the pipeline logs, so a scenario can check one was logged.
pub struct ErrorLog(pub Mutex<Vec<String>>);

impl log::Log for ErrorLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Error
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.0.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}
