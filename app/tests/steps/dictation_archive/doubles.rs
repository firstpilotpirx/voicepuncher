//! The stand-ins of the dictation archive scenarios.

use super::{
    anyhow, joined, mpsc, Answers, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, Event,
    Events, FailureNotice, Instant, LiveChunk, RawTranscript, Receiver, Result, Sender,
    SoundPlayer, Takes, TextPaster, Transcriber, VisualIndicator,
};

/// Hands over the audio queued for each recording; recorded in chunks, it sends all parts
/// but the last while the user speaks and the last one when the recording is flushed.
pub(super) struct TakesRecorder {
    pub(super) takes: Takes,
    pub(super) in_chunks: bool,
    pub(super) current: Vec<AudioBuffer16k>,
    pub(super) parts: Option<Sender<LiveChunk>>,
}

impl TakesRecorder {
    pub(super) fn send(&self, audio: &AudioBuffer16k) {
        if let Some(parts) = &self.parts {
            let _ = parts.send(LiveChunk { audio: audio.clone(), enqueued_at: Instant::now() });
        }
    }
}

impl AudioRecorder for TakesRecorder {
    fn start(&mut self) -> Result<()> {
        self.current = self.takes.lock().unwrap().pop_front().unwrap_or_default();
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        self.parts = None;
        Ok(joined(&self.current))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.in_chunks {
            return Ok(None);
        }
        let (tx, rx) = mpsc::channel();
        self.parts = Some(tx);
        let spoken = self.current.len().saturating_sub(1);
        for part in &self.current[..spoken] {
            self.send(part);
        }
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        if let Some(last) = self.current.last() {
            self.send(last);
        }
        // the transcription of the parts ends with the last one
        self.parts = None;
    }

    fn recording(&mut self) -> Option<AudioBuffer16k> {
        Some(joined(&self.current))
    }
    fn truncate_at(&mut self, _cut: Instant) {}
    fn begin_with(&mut self, _lead_in: AudioBuffer16k) {}
}

pub(super) struct AnsweringTranscriber {
    pub(super) answers: Answers,
    pub(super) fails: bool,
}

impl Transcriber for AnsweringTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let answer = self.answers.lock().unwrap().pop_front();
        if self.fails {
            return Err(anyhow!("the speech recognition service does not answer"));
        }
        Ok(answer.map(RawTranscript::new).transpose()?)
    }
}

pub(super) struct NotingPaster(pub(super) Events);

impl TextPaster for NotingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .push(Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct NotingOverlay(pub(super) Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_waiting_for_focus(&self) {}

    fn hide(&self) {
        self.0.lock().unwrap().push(Event::Hidden);
    }

    /// The Pasted sign takes itself down: the dictation ends on the overlay with it.
    fn show_pasted(&self) {
        self.hide();
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        self.hide();
    }
    voicepuncher::shows_no_voice_control_signs!();
}

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}
