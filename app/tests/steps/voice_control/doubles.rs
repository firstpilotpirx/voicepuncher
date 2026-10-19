//! Stand-ins for the ports the pipeline drives in the voice-control scenarios. Each records
//! what it was asked to do as an `Event` in one shared, ordered log.

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use anyhow::Result;

use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, SoundPlayer, TextPaster, Transcriber,
    VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, RawTranscript, VoiceCommand,
};

/// The stand-ins' audio rate: one sample per millisecond is enough to measure a lead-in.
const SAMPLES_PER_SEC: f32 = AudioBuffer16k::SAMPLE_RATE_HZ as f32;

/// Long enough that a command spoken right after "stop dictation" arrives before the paste.
pub const TRANSCRIPTION_TIME: Duration = Duration::from_millis(150);
/// How late the pipeline's thread records the last event of a reaction, as a busy CPU makes it.
/// It stays so a step that reads the log right after an earlier event of the reaction fails on
/// every run instead of now and then.
const PIPELINE_LAG: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    RecordingStarted,
    RecordingCancelled,
    Pasted(String),
    ReturnPressed,
    AllTextCleared,
    PasteKeyPressed,
    Sound(&'static str),
    ListeningStarted,
    ListeningStopped,
    Overlay(&'static str),
}

pub type Events = Arc<Mutex<Vec<Event>>>;

/// What the user said, with the moment each part started; a recording that starts drops
/// what was said before it.
pub type Speech = Arc<Mutex<Vec<(Instant, String)>>>;

pub type Cut = Arc<Mutex<Option<Instant>>>;

/// The phrases the command spotting service listens for right now.
pub type Phrases = Arc<Mutex<Vec<VoiceCommand>>>;

pub fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

pub struct NotingRecorder {
    pub events: Events,
    pub speech: Speech,
    pub cut: Cut,
    /// What was said before the recording started: a lead-in may bring some of it back.
    pub said_before: Vec<(Instant, String)>,
}

impl AudioRecorder for NotingRecorder {
    fn start(&mut self) -> Result<()> {
        self.said_before = std::mem::take(&mut *self.speech.lock().unwrap());
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

    /// Brings back what was said within the lead-in, which reaches back from now, so it
    /// is transcribed with the recording.
    fn begin_with(&mut self, lead_in: AudioBuffer16k) {
        let now = Instant::now();
        let heard_from = now - Duration::from_millis(lead_in.duration_ms());
        let heard: Vec<(Instant, String)> = self
            .said_before
            .drain(..)
            .filter(|(at, _)| (heard_from..=now).contains(at))
            .collect();
        self.speech.lock().unwrap().splice(0..0, heard);
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

    fn paste(&mut self) -> Result<()> {
        record(&self.0, Event::PasteKeyPressed);
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
        std::thread::sleep(PIPELINE_LAG);
        record(&self.0, Event::Sound("listening on"));
    }

    fn play_listening_off(&self) {
        record(&self.0, Event::Sound("listening off"));
    }

    fn play_enter(&self) {
        record(&self.0, Event::Sound("enter"));
    }

    fn play_clear(&self) {
        record(&self.0, Event::Sound("clear"));
    }

    fn play_paste(&self) {
        record(&self.0, Event::Sound("paste"));
    }

    fn play_restart(&self) {
        record(&self.0, Event::Sound("restart"));
    }
}

pub struct RecordingOverlay(pub Events);

impl VisualIndicator for RecordingOverlay {
    fn show_command_sign(&self, command: VoiceCommand) {
        let sign = match command {
            VoiceCommand::PressEnter => "enter sign",
            VoiceCommand::ClearAllText => "clear sign",
            VoiceCommand::PasteText => "paste sign",
            VoiceCommand::RestartDictation => "restart sign",
            _ => "other sign",
        };
        record(&self.0, Event::Overlay(sign));
    }

    fn show_recording(&self) {
        record(&self.0, Event::Overlay("recording"));
    }

    fn show_recognizing_speech(&self) {
        record(&self.0, Event::Overlay("recognizing speech"));
    }

    fn show_refining_text(&self) {
        record(&self.0, Event::Overlay("refining text"));
    }

    fn show_pasted(&self) {
        record(&self.0, Event::Overlay("pasted sign"));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.0, Event::Overlay("waiting for focus"));
    }

    fn hide(&self) {
        record(&self.0, Event::Overlay("hidden"));
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        record(&self.0, Event::Overlay("failure notice"));
    }

    fn show_commands_unavailable(&self) {
        record(&self.0, Event::Overlay("commands unavailable shown"));
    }

    fn hide_commands_unavailable(&self) {
        record(&self.0, Event::Overlay("commands unavailable hidden"));
    }
}

/// The command spotting service as the pipeline sees it; the scenario feeds its events
/// straight into the pipeline's input channel, and reports a phrase only while the
/// service listens for it, as the real service does.
pub struct ScriptedSpotter {
    pub events: Events,
    pub phrases: Phrases,
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

    fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        let secs = Instant::now()
            .saturating_duration_since(since)
            .as_secs_f32();
        AudioBuffer16k::new(vec![0; (secs * SAMPLES_PER_SEC) as usize])
    }
}

/// The start of the line the pipeline logs at every voice start.
pub const VOICE_START_LINE: &str = "voice start:";

/// Keeps every error the pipeline logs, and every warning and voice start timing line with
/// the thread that logged it, so a scenario can check what its own pipeline logged.
#[derive(Default)]
pub struct PipelineLog {
    pub errors: Mutex<Vec<String>>,
    pub warnings: Mutex<Vec<(ThreadId, String)>>,
    pub voice_starts: Mutex<Vec<(ThreadId, String)>>,
}

impl log::Log for PipelineLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        match record.level() {
            log::Level::Error => self.errors.lock().unwrap().push(record.args().to_string()),
            log::Level::Warn => {
                let thread = std::thread::current().id();
                let line = record.args().to_string();
                self.warnings.lock().unwrap().push((thread, line));
            }
            log::Level::Info => {
                let line = record.args().to_string();
                if line.starts_with(VOICE_START_LINE) {
                    let thread = std::thread::current().id();
                    self.voice_starts.lock().unwrap().push((thread, line));
                }
            }
            _ => {}
        }
    }

    fn flush(&self) {}
}
