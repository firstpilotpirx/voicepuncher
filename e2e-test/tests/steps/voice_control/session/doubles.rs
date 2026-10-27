//! The stand-ins of the session scenarios.

#![allow(unused_imports)]

use super::{
    Arc, AudioBuffer16k, AudioRecorder, AudioStream, ChunkingCfg, CleanedText, CommandSpotter,
    Context, Cues, Event, Events, FailureNotice, Instant, KeyPresser, LiveChunk, Microphone,
    MicrophoneAudio, PipelineInput, Receiver, Result, Sender, SoundPlayer, TextPaster,
    VisualIndicator, VoiceCommand, MICROPHONE_OPENING, SAMPLE_RATE_HZ,
};
use voicepuncher_e2e_test::conversions::{saturating_i16, saturating_u32};

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
            .map(|s| saturating_i16(s.clamp(-1.0, 1.0) * f32::from(i16::MAX)));
        let samples = self.lead_in.iter().copied().chain(recorded).collect();
        Ok(AudioBuffer16k::new(samples))
    }

    voicepuncher::no_live_chunks!();

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

/// The events, the start cues and the stop sounds.
pub(super) struct RecordingSounds(pub(super) Events, pub(super) Cues, pub(super) Cues);

impl SoundPlayer for RecordingSounds {
    fn play_start(&self) {
        self.1.lock().unwrap().push(Instant::now());
    }
    fn play_stop(&self) {
        self.2.lock().unwrap().push(Instant::now());
    }
    fn play_cancel(&self) {
        self.0.lock().unwrap().push(Event::Cancelled);
    }
    fn play_restart(&self) {
        self.1.lock().unwrap().push(Instant::now());
    }
}

pub(super) struct NoOverlay;

impl VisualIndicator for NoOverlay {
    voicepuncher::shows_no_dictation_states!();
    voicepuncher::shows_no_voice_control_signs!();
}
