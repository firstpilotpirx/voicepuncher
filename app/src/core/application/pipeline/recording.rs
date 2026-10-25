//! A dictation's recording: its start, its stop into processing, its cancel and its abandon.

use std::sync::{mpsc, Arc};
use std::time::Instant;

use crate::core::application::chunking::merge_chunk_texts;
use crate::core::application::DictationOutcome;
use crate::core::domain::{FailingPart, FailureNotice, RawTranscript};

use super::archiving::{dictation_start, no_text_outcome};
use super::dictation::Dictation;
use super::live_transcript::LiveTranscript;
use super::live_worker::run_live_worker;
use super::processing::StageClock;
use super::{LiveSessionResult, LiveTranscriptionSession, Pipeline, StartCue};

/// What an abandoned dictation's end waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Awaits {
    /// The live worker's last result, so the dictation ends with the worker idle.
    WorkerResult,
    /// Nothing: the worker still transcribing the last chunk finishes for nobody.
    Nothing,
}

impl Pipeline {
    /// Starts a recording; one started by voice begins at `begins_at`, the end of the
    /// command, with the speech heard since then.
    pub(super) fn start_recording_from(
        &mut self,
        begins_at: Option<Instant>,
        cue: StartCue,
    ) -> anyhow::Result<()> {
        // With voice control the dictation phrases are listened for before the recording
        // starts, so a "stop dictation" said right away is heard.
        self.voice.follow_dictation(true);
        match cue {
            StartCue::Start => self.sound_player.play_start(),
            StartCue::Restart => self.sound_player.play_restart(),
        }
        if let Err(e) = self.recorder.start() {
            self.voice.follow_dictation(false);
            return Err(self.recording_not_started(e));
        }
        if let Some(since) = begins_at {
            // Taken once the recorder runs, so the heard speech reaches its first sample,
            // and before live chunks, so they count the speech as the recording's start.
            let lead_in = self.voice.heard_since(since);
            self.recorder.begin_with(lead_in);
        }
        let mut dictation = self.new_dictation(dictation_start(begins_at, self.cfg.recording_mode));
        if self.cfg.chunking.chunk_threshold_secs > 0 {
            dictation.live = self.start_live_session()?;
        }
        self.visual_indicator.show_recording();
        self.dictation = Some(dictation);
        Ok(())
    }

    /// Starts transcribing the recording in chunks as it goes; none when the recorder
    /// records whole buffers only.
    fn start_live_session(&mut self) -> anyhow::Result<Option<LiveTranscriptionSession>> {
        let chunking = self.cfg.chunking;
        let Some(chunk_rx) = self.recorder.start_live_chunks(chunking)? else {
            return Ok(None);
        };
        log::info!(
            "live chunking: threshold={}s overlap={}s → target chunk duration ≈ {}s",
            chunking.chunk_threshold_secs,
            chunking.chunk_overlap_secs,
            chunking.chunk_threshold_secs + chunking.chunk_overlap_secs,
        );
        let transcriber = Arc::clone(&self.transcriber);
        let (results_tx, results_rx) = mpsc::sync_channel(1);
        let (live_transcript, recording_end) =
            LiveTranscript::new(self.visual_indicator.live_transcript_view());
        std::thread::Builder::new()
            .name("live-transcription".into())
            .spawn(move || {
                run_live_worker(&transcriber, &chunk_rx, &results_tx, &live_transcript);
            })?;
        Ok(Some(LiveTranscriptionSession { results_rx, recording_end }))
    }

    /// Stops the running dictation and processes it: nothing when none runs.
    pub(super) fn stop_recording(&mut self) -> anyhow::Result<()> {
        let t_user_stop = Instant::now();
        let Some(mut dictation) = self.dictation.take() else {
            return Ok(());
        };
        self.voice.follow_dictation(false);
        self.sound_player.play_stop();
        self.visual_indicator.show_recognizing_speech();
        let clock = StageClock::since(t_user_stop);
        match dictation.live.take() {
            Some(session) => self.stop_live(dictation, &session, clock),
            None => return self.stop_whole(dictation, clock),
        }
        Ok(())
    }

    /// Stops a dictation transcribed on stop: the whole recording goes to speech recognition.
    fn stop_whole(
        &mut self,
        mut dictation: Dictation,
        mut clock: StageClock,
    ) -> anyhow::Result<()> {
        let audio = self.recorder.stop()?;
        clock.lap("stop");
        if audio.is_empty() || audio.duration_ms() < self.cfg.min_press_ms {
            log::info!("dropped: too short ({} ms)", audio.duration_ms());
            self.end_empty_cycle_on_overlay(&mut dictation);
            return Ok(());
        }
        let pasting = format!("({:.1}s)", audio.duration_ms() as f32 / 1000.0);
        match self.transcribe_recording(&mut dictation, audio, &mut clock) {
            Ok(raw) => self.finish(dictation, &raw, clock, &pasting),
            Err(outcome) => self.end_without_text(dictation, outcome, &clock),
        }
        Ok(())
    }

    /// Stops a dictation transcribed in chunks: its worker's last result is waited for,
    /// then merged.
    fn stop_live(
        &mut self,
        mut dictation: Dictation,
        session: &LiveTranscriptionSession,
        mut clock: StageClock,
    ) {
        session.recording_end.signal();
        self.recorder.flush_live_chunks();
        let result = match session.results_rx.recv_timeout(self.cfg.stop_wait) {
            Ok(result) => result,
            Err(e) => {
                self.log_lost_live_result(e);
                dictation
                    .note_failure(FailureNotice::DictationLost(FailingPart::SpeechRecognition));
                self.stop_recorder_quietly("after a lost live result");
                self.end_without_text(dictation, DictationOutcome::SpeechRecognitionFailed, &clock);
                return;
            }
        };
        clock.lap("drain");
        self.stop_recorder_quietly("after the live result");
        clock.lap("stop");
        dictation.note_lost_chunks(result.lost_chunks, result.texts.is_empty());
        dictation.trace_chunks(&result.texts, result.lost_chunks);
        match merge_live(&mut dictation, &result, &mut clock) {
            Ok(raw) => self.finish(dictation, &raw, clock, "(live)"),
            Err(outcome) => self.end_without_text(dictation, outcome, &clock),
        }
    }

    fn log_lost_live_result(&self, error: mpsc::RecvTimeoutError) {
        match error {
            mpsc::RecvTimeoutError::Timeout => log::error!(
                "live transcription: no transcript within {:?} of the stop, dictation dropped",
                self.cfg.stop_wait
            ),
            mpsc::RecvTimeoutError::Disconnected => {
                log::error!(
                    "live transcription worker stopped without a result, dictation dropped"
                );
            }
        }
    }

    pub(super) fn cancel_recording(&mut self) -> anyhow::Result<()> {
        self.abandon(Awaits::WorkerResult)?;
        self.sound_player.play_cancel();
        self.visual_indicator.hide();
        log::info!("recording cancelled");
        Ok(())
    }

    /// Stops the recording and drops it, with no cue: the caller gives its own.
    pub(super) fn abandon(&mut self, awaits: Awaits) -> anyhow::Result<()> {
        self.voice.follow_dictation(false);
        let Some(session) = self.dictation.take().and_then(|dictation| dictation.live) else {
            self.recorder.stop()?;
            return Ok(());
        };
        session.recording_end.signal();
        if awaits == Awaits::WorkerResult {
            self.recorder.flush_live_chunks();
        }
        self.stop_recorder_quietly("of an abandoned dictation");
        if awaits == Awaits::WorkerResult
            && session.results_rx.recv_timeout(self.cfg.stop_wait).is_err()
        {
            log::error!("live worker gave no result on cancel");
        }
        Ok(())
    }

    /// Stops the recorder when its recording is no longer needed; a failure is only logged.
    pub(super) fn stop_recorder_quietly(&mut self, why: &str) {
        if let Err(e) = self.recorder.stop() {
            log::warn!("recorder not stopped {why}: {e:#}");
        }
    }
}

/// The merged transcript of a live session's chunks, or how the dictation ends without one.
fn merge_live(
    dictation: &mut Dictation,
    result: &LiveSessionResult,
    clock: &mut StageClock,
) -> Result<RawTranscript, DictationOutcome> {
    if result.texts.is_empty() {
        log::info!("live session: no text ({})", clock.line());
        return Err(no_text_outcome(result.lost_chunks));
    }
    let merged = merge_chunk_texts(&result.texts);
    clock.lap("merge");
    log::info!(
        "live session: {} chars merged ({})",
        merged.len(),
        clock.line()
    );
    RawTranscript::new(dictation.without_command_words(&merged)).map_err(|e| {
        log::warn!("live session: bad transcript: {e}");
        DictationOutcome::OnlyCommandWords
    })
}
