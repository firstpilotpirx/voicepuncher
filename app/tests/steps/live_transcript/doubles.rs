//! The stand-ins of the live transcript scenarios.

use super::{
    mpsc, send_part, Arc, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, FailureNotice,
    LiveChunk, LiveTranscriptView, Log, Mutex, Parts, RawTranscript, Receiver, Result, Shown,
    SoundPlayer, TextPaster, Transcriber, VecDeque, VisualIndicator,
};
use voicepuncher::core::domain::VoiceCommand;

/// Hands over a part only when the scenario sends one; the last part comes at the flush.
pub(super) struct PartsOnDemand(pub(super) Parts);

impl AudioRecorder for PartsOnDemand {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(Vec::new()))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        let (tx, rx) = mpsc::channel();
        *self.0.lock().unwrap() = Some(tx);
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        let _ = send_part(&self.0);
        // the transcription of the parts ends with the last one
        *self.0.lock().unwrap() = None;
    }
    voicepuncher::records_whole_buffers_only!();
}

/// Answers each part with the next scripted text; `None` is a part without speech.
pub(super) struct ScriptedTranscriber {
    pub(super) answers: Arc<Mutex<VecDeque<Option<&'static str>>>>,
    pub(super) transcribed: Arc<Mutex<usize>>,
}

impl Transcriber for ScriptedTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let answer = self.answers.lock().unwrap().pop_front().flatten();
        let raw = answer
            .map(|text| RawTranscript::new(text.to_owned()))
            .transpose()?;
        *self.transcribed.lock().unwrap() += 1;
        Ok(raw)
    }
}

pub(super) struct CapturingPaster(pub(super) Arc<Mutex<Option<String>>>);

impl TextPaster for CapturingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        *self.0.lock().unwrap() = Some(text.as_str().to_owned());
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct NotingOverlay(pub(super) Log);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        self.0.lock().unwrap().push(Shown::Recording);
    }

    fn show_recognizing_speech(&self) {
        self.0.lock().unwrap().push(Shown::RecognizingSpeech);
    }

    fn show_refining_text(&self) {
        self.0.lock().unwrap().push(Shown::Other);
    }

    fn show_pasted(&self) {
        self.0.lock().unwrap().push(Shown::PastedSign);
    }

    fn show_waiting_for_focus(&self) {
        self.0.lock().unwrap().push(Shown::Other);
    }

    fn hide(&self) {
        self.0.lock().unwrap().push(Shown::Hidden);
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        self.0.lock().unwrap().push(Shown::Other);
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(NotingView(Arc::clone(&self.0)))
    }
    fn show_command_sign(&self, _command: VoiceCommand) {}
    fn show_commands_unavailable(&self) {}
    fn hide_commands_unavailable(&self) {}
}

pub(super) struct NotingView(pub(super) Log);

impl LiveTranscriptView for NotingView {
    fn show_live_transcript(&self, text: &str) {
        self.0
            .lock()
            .unwrap()
            .push(Shown::LiveTranscript(text.to_owned()));
    }
}

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}
