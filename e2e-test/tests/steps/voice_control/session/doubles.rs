//! The stand-ins of the session scenarios.

#![allow(unused_imports)]

use super::{
    Arc, AudioBuffer16k, AudioRecorder, AudioStream, ChunkingCfg, CleanedText, CommandSpotter,
    Context, Cues, Event, Events, FailureNotice, Instant, KeyPresser, LiveChunk, Microphone,
    PipelineInput, Receiver, Result, Sender, SoundPlayer, TextPaster, VisualIndicator,
    VoiceCommand, MICROPHONE_OPENING, SAMPLE_RATE_HZ,
};

/// The command spotting adapter's streaming path, fed by the session's microphone.
pub(super) struct MicrophoneSpotter {
    pub(super) url: String,
    pub(super) inputs: Sender<PipelineInput>,
    pub(super) heard: Option<Receiver<Vec<f32>>>,
    pub(super) stream: Option<AudioStream>,
    pub(super) microphone: Arc<Microphone>,
    pub(super) events: Events,
}

impl CommandSpotter for MicrophoneSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let heard = self
            .heard
            .take()
            .context("the microphone is already in use")?;
        let started_at = Instant::now();
        *self.microphone.started_at.lock().unwrap() = Some(started_at);
        let rate = SAMPLE_RATE_HZ as u32;
        let stream = AudioStream::start(
            &self.url,
            commands,
            started_at,
            rate,
            heard,
            self.inputs.clone(),
        )?;
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
        self.stream.as_ref().map_or_else(
            || AudioBuffer16k::new(Vec::new()),
            |stream| stream.heard_since(since),
        )
    }
}

/// Records from the session's microphone, as the real recorder records from the real one.
pub(super) struct MicrophoneRecorder {
    pub(super) microphone: Arc<Microphone>,
    pub(super) events: Events,
    pub(super) start: usize,
    pub(super) cut: Option<usize>,
    /// Heard before the recording started, put at its beginning.
    pub(super) lead_in: Vec<i16>,
}

impl AudioRecorder for MicrophoneRecorder {
    fn start(&mut self) -> Result<()> {
        std::thread::sleep(MICROPHONE_OPENING);
        self.start = self.microphone.captured.lock().unwrap().len();
        self.cut = None;
        self.lead_in.clear();
        self.events.lock().unwrap().push(Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let captured = self.microphone.captured.lock().unwrap();
        let end = self.cut.unwrap_or(captured.len()).min(captured.len());
        let recorded = captured[self.start.min(end)..end]
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16);
        let samples = self.lead_in.iter().copied().chain(recorded).collect();
        Ok(AudioBuffer16k::new(samples))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}

    fn truncate_at(&mut self, cut: Instant) {
        self.cut = Some(self.microphone.index_at(cut));
    }

    fn begin_with(&mut self, lead_in: AudioBuffer16k) {
        self.lead_in = lead_in.samples().to_vec();
    }
    fn recording(&mut self) -> Option<voicepuncher::core::domain::AudioBuffer16k> {
        None
    }
}

pub(super) struct RecordingPaster(pub(super) Events);

impl TextPaster for RecordingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        let pasted = Event::Pasted(text.as_str().to_owned());
        self.0.lock().unwrap().push(pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct RecordingKeys(pub(super) Events);

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

pub(super) struct RecordingSounds(pub(super) Events, pub(super) Cues);

impl SoundPlayer for RecordingSounds {
    fn play_start(&self) {
        self.1.lock().unwrap().push(Instant::now());
    }
    fn play_stop(&self) {}
    fn play_cancel(&self) {
        self.0.lock().unwrap().push(Event::Cancelled);
    }
    fn play_restart(&self) {
        self.1.lock().unwrap().push(Instant::now());
    }
}

pub(super) struct NoOverlay;

impl VisualIndicator for NoOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
    voicepuncher::shows_no_voice_control_signs!();
}
