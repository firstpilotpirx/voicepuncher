//! The stand-ins of the speech recognition scenarios.

use super::{
    bail, mpsc, Arc, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, Context, Instant,
    LiveChunk, Mutex, RawTranscript, Receiver, Result, TextPaster, Transcriber,
};

/// Delivers `chunks` one-second live chunks; the first sample numbers the chunk.
pub(super) struct ChunkingRecorder {
    pub(super) chunks: usize,
}

impl AudioRecorder for ChunkingRecorder {
    voicepuncher::records_nothing!();

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        let (tx, rx) = mpsc::channel();
        for n in 0..self.chunks {
            let audio = voicepuncher::test_doubles::numbered_second(n);
            tx.send(LiveChunk { audio, enqueued_at: Instant::now() })?;
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
        let n = usize::try_from(audio.samples()[0]).unwrap();
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

pub(super) use voicepuncher::test_doubles::Silent;
