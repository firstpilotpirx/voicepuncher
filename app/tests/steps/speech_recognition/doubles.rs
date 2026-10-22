//! The stand-ins of the speech recognition scenarios.

use super::{
    bail, mpsc, Arc, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, Context,
    FailureNotice, Instant, LiveChunk, Mutex, RawTranscript, Receiver, Result, SoundPlayer,
    TextPaster, Transcriber, VisualIndicator,
};

/// Delivers `chunks` one-second live chunks; the first sample numbers the chunk.
pub(super) struct ChunkingRecorder {
    pub(super) chunks: usize,
}

impl AudioRecorder for ChunkingRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(Vec::new()))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        let (tx, rx) = mpsc::channel();
        for n in 0..self.chunks {
            let mut samples = vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize];
            samples[0] = n as i16;
            tx.send(LiveChunk {
                audio: AudioBuffer16k::new(samples),
                enqueued_at: Instant::now(),
            })?;
        }
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {}
    voicepuncher::records_whole_buffers_only!();
}

/// Answers chunk n with the n-th transcript, and fails the chunk it is told to fail.
pub(super) struct FixedTranscripts(pub(super) Vec<&'static str>, pub(super) Option<usize>);

impl Transcriber for FixedTranscripts {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let n = audio.samples()[0] as usize;
        if self.1 == Some(n) {
            bail!("speech recognition request failed: every attempt timed out");
        }
        let text = self.0.get(n).context("no transcript for this chunk")?;
        Ok(Some(RawTranscript::new((*text).to_owned())?))
    }
}

pub(super) struct CapturingPaster(pub(super) Arc<Mutex<Vec<String>>>);

impl TextPaster for CapturingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.0.lock().unwrap().push(text.as_str().to_owned());
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        bail!("the text was left in the clipboard instead of pasted")
    }
}

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}

impl VisualIndicator for Silent {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
    voicepuncher::shows_no_voice_control_signs!();
}
