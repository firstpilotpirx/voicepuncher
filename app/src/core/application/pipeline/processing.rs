//! What a stopped dictation goes through: speech recognition, the filler-sound cleanup,
//! delivery, the overlay's end and the archive entry; file mode runs the same stages once.

use crate::core::domain::conversions::millis_u64;
use std::time::Instant;

use crate::core::application::chunking::{merge_chunk_texts, split_into_chunks};
use crate::core::application::{
    insert_text, DictationOutcome, DictationStart, FocusWait, FocusWaitCfg, StageTiming,
};
use crate::core::domain::{
    text_to_paste, AudioBuffer16k, CleanedText, FailureNotice, RawTranscript,
};

use super::archiving::insertion_outcome;
use super::dictation::Dictation;
use super::failure_notices::{not_pasted, recognition_part};
use super::{lock_transcriber, Pipeline};

/// The named stages a dictation ran since the user stopped it, in order, each with the
/// time it took, as the `stop→paste` log line names them.
pub(super) struct StageClock {
    started: Instant,
    stage_started: Instant,
    stages: Vec<StageTiming>,
}

impl StageClock {
    /// A clock started at `started`, the user's stop; its first stage starts now.
    pub(super) fn since(started: Instant) -> Self {
        Self { started, stage_started: Instant::now(), stages: Vec::new() }
    }

    pub(super) fn start() -> Self {
        Self::since(Instant::now())
    }

    /// Ends `stage` now and starts the next one.
    pub(super) fn lap(&mut self, stage: &'static str) {
        let ms = millis_u64(self.stage_started.elapsed());
        self.stages.push(StageTiming { stage, ms });
        self.stage_started = Instant::now();
    }

    /// Ends the clock: the total since the user's stop is its last stage.
    fn total(&mut self) {
        let ms = millis_u64(self.started.elapsed());
        self.stages.push(StageTiming { stage: "total", ms });
    }

    pub(super) fn stages(&self) -> &[StageTiming] {
        &self.stages
    }

    /// The stages as the log names them: `stop=3ms transcribe=400ms …`.
    pub(super) fn line(&self) -> String {
        self.stages
            .iter()
            .map(|timing| format!("{}={}ms", timing.stage, timing.ms))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Pipeline {
    /// Delivers `raw` without its filler sounds, ends the dictation on the overlay, logs the
    /// stages and keeps the dictation in the archive; `pasting` names the dictation in the
    /// paste log.
    pub(super) fn finish(
        &mut self,
        mut dictation: Dictation,
        raw: &RawTranscript,
        mut clock: StageClock,
        pasting: &str,
    ) {
        let cleaned = text_to_paste(raw);
        dictation.trace_transcript(raw, cleaned.as_ref());
        let Some(cleaned) = cleaned else {
            return self.end_without_text(dictation, DictationOutcome::NoText, &clock);
        };
        log::info!("pasting… {pasting}");
        let outcome = self.deliver(&mut dictation, &cleaned);
        clock.lap("paste");
        clock.total();
        self.end_delivery_on_overlay(&mut dictation, outcome);
        log::info!("stop→paste: {}", clock.line());
        self.close_trace(dictation, outcome, clock.stages());
    }

    /// Ends a dictation that yields no text to paste: on the overlay and in the archive.
    pub(super) fn end_without_text(
        &mut self,
        mut dictation: Dictation,
        outcome: DictationOutcome,
        clock: &StageClock,
    ) {
        self.end_cycle_on_overlay(&mut dictation);
        self.close_trace(dictation, outcome, clock.stages());
    }

    /// The transcript of `audio` to paste, or how the dictation ends without one.
    pub(super) fn transcribe_recording(
        &self,
        dictation: &mut Dictation,
        audio: AudioBuffer16k,
        clock: &mut StageClock,
    ) -> Result<RawTranscript, DictationOutcome> {
        log::info!("transcribing…");
        let transcribed = lock_transcriber(&self.transcriber).transcribe(&audio);
        clock.lap("transcribe");
        dictation.trace_audio(audio);
        let raw = match transcribed {
            Err(e) => {
                log::warn!("transcription failed, dropping cycle: {e:#}");
                dictation.note_failure(FailureNotice::DictationLost(recognition_part(&e)));
                return Err(DictationOutcome::SpeechRecognitionFailed);
            }
            Ok(None) => return Err(DictationOutcome::NoText),
            Ok(Some(raw)) => raw,
        };
        dictation.trace_chunks(&[raw.as_str().to_owned()], 0);
        RawTranscript::new(dictation.without_command_words(raw.as_str())).map_err(|_| {
            log::info!("dropped: only command words were transcribed");
            DictationOutcome::OnlyCommandWords
        })
    }

    /// Pastes `cleaned`, after a wait for text input focus when one is configured.
    fn deliver(&mut self, dictation: &mut Dictation, cleaned: &CleanedText) -> DictationOutcome {
        let insertion = insert_text(
            cleaned,
            self.paster.as_mut(),
            FocusWait {
                probe: self.focus.as_ref().map(|focus| focus.probe.as_ref()),
                cfg: self
                    .focus
                    .as_ref()
                    .map_or(FocusWaitCfg::OFF, |focus| focus.cfg),
            },
            self.visual_indicator.as_ref(),
            &mut || self.inbox.hotkey_pressed(),
        );
        if let Err(e) = &insertion {
            log::error!("paste failed, dropping cycle: {e:#}");
            dictation.note_failure(not_pasted(e));
        }
        insertion_outcome(&insertion)
    }

    /// Transcribes and pastes one recording, as file mode does; the pasted text,
    /// none when the recording held no speech.
    pub fn run_once(&mut self, audio: AudioBuffer16k) -> anyhow::Result<Option<CleanedText>> {
        let mut dictation = self.new_dictation(DictationStart::RecordingFile);
        let chunks = split_into_chunks(&audio, self.cfg.chunking);
        if chunks.len() > 1 {
            log::info!("file mode: {} chunks", chunks.len());
        }
        dictation.trace_audio(audio);
        let mut clock = StageClock::start();
        let texts = match self.transcribe_chunks(&chunks) {
            Ok(texts) => texts,
            Err(e) => {
                self.close_trace(dictation, DictationOutcome::SpeechRecognitionFailed, &[]);
                return Err(e);
            }
        };
        clock.lap("transcribe");
        dictation.trace_chunks(&texts, 0);
        let raw = RawTranscript::new(merge_chunk_texts(&texts)).ok();
        let cleaned = raw.as_ref().and_then(text_to_paste);
        if let Some(raw) = &raw {
            dictation.trace_transcript(raw, cleaned.as_ref());
        }
        let Some(cleaned) = cleaned else {
            self.close_trace(dictation, DictationOutcome::NoText, clock.stages());
            return Ok(None);
        };
        let pasted = self.paster.paste(&cleaned);
        let outcome = match pasted {
            Ok(()) => DictationOutcome::Pasted,
            Err(_) => DictationOutcome::PasteFailed,
        };
        self.close_trace(dictation, outcome, clock.stages());
        pasted?;
        Ok(Some(cleaned))
    }

    /// The text speech recognition returned for each chunk that had any.
    fn transcribe_chunks(&self, chunks: &[AudioBuffer16k]) -> anyhow::Result<Vec<String>> {
        let mut transcriber = lock_transcriber(&self.transcriber);
        let mut texts = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            if let Some(raw) = transcriber.transcribe(chunk)? {
                texts.push(raw.as_str().to_owned());
            }
        }
        Ok(texts)
    }
}
