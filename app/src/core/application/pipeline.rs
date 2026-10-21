mod archiving;
mod failure_notices;
mod live_transcript;
mod voice_control;

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Instant;

use crate::core::application::audio_recorder::LiveChunk;
use crate::core::application::chunking::{merge_chunk_texts, split_into_chunks, ChunkingCfg};
use crate::core::application::pipeline::archiving::{
    dictation_start, insertion_outcome, no_text_outcome, timings, DictationTrace,
};
use crate::core::application::pipeline::failure_notices::{not_pasted, past_recording_not_started};
use crate::core::application::pipeline::live_transcript::{LiveTranscript, RecordingEnd};
use crate::core::application::text_refiner::refine_or_raw;
use crate::core::application::{
    hotkey_pressed_during_wait, insert_text, AudioRecorder, DictationArchive, DictationOutcome,
    DictationStart, FocusProbe, FocusWaitCfg, Insertion, NoActivitySink, PipelineActivity,
    PipelineActivitySink, PipelineInput, SoundPlayer, StageTiming, TextPaster, TextRefiner,
    Transcriber, VisualIndicator,
};

use crate::core::domain::{
    without_leading_command, without_trailing_command, AudioBuffer16k, CleanedText, FailingPart,
    FailureNotice, Permission, PttSignal, RawTranscript, RecordingMode, VoiceCommand,
};

#[derive(Debug, Clone, Copy)]
pub struct PipelineConfig {
    pub min_press_ms: u64,
    pub recording_mode: RecordingMode,
    pub chunking: ChunkingCfg,
}

/// The cue a recording starts with, played before the recorder opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartCue {
    Start,
    Restart,
}

#[derive(Debug, Clone, Copy)]
enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    Refining,
    Pasting,
}

struct LiveTranscriptionSession {
    results_rx: mpsc::Receiver<LiveSessionResult>,
    recording_end: RecordingEnd,
}

struct LiveSessionResult {
    texts: Vec<String>,
    /// Chunks speech recognition failed on.
    lost_chunks: usize,
    transcriber: Box<dyn Transcriber>,
}

/// Where a live worker gets the transcriber from: at once, or from the worker of a dropped
/// dictation once its last request ends.
enum TranscriberSource {
    Ready(Box<dyn Transcriber>),
    ReturnedBy(mpsc::Receiver<LiveSessionResult>),
}

impl TranscriberSource {
    /// The transcriber, once it is back; none when the dropped dictation's worker died.
    fn take(self) -> Option<Box<dyn Transcriber>> {
        match self {
            TranscriberSource::Ready(transcriber) => Some(transcriber),
            TranscriberSource::ReturnedBy(results) => results.recv().ok().map(|r| r.transcriber),
        }
    }
}

pub struct Pipeline {
    pub recorder: Box<dyn AudioRecorder>,
    pub transcriber: Option<Box<dyn Transcriber>>,
    pub refiner: Box<dyn TextRefiner>,
    pub paster: Box<dyn TextPaster>,
    pub sound_player: Box<dyn SoundPlayer>,
    pub visual_indicator: Box<dyn VisualIndicator>,
    pub cfg: PipelineConfig,
    state: PipelineState,
    live_session: Option<LiveTranscriptionSession>,
    /// The worker of a dictation "restart dictation" dropped, which returns the transcriber
    /// once its last request ends.
    dropped_worker: Option<mpsc::Receiver<LiveSessionResult>>,
    focus_probe: Option<Box<dyn FocusProbe>>,
    focus_wait: FocusWaitCfg,
    inputs: Option<mpsc::Receiver<PipelineInput>>,
    /// Inputs read during a focus wait, handled before the channel is read again.
    pending: VecDeque<PipelineInput>,
    voice: Option<voice_control::VoiceControl>,
    /// Command words the running dictation may hold in its text.
    command_words: CommandWords,
    /// Whether the overlay shows the text refinement stage: off when refinement is off.
    shows_refinement: bool,
    /// The failure notice the running cycle ends with instead of hiding the overlay.
    pending_notice: Option<FailureNotice>,
    archive: Option<Box<dyn DictationArchive>>,
    /// The running dictation as its archive entry will hold it; none without an archive.
    trace: Option<DictationTrace>,
    /// Where the activity (idle, recording, processing) is published after every input.
    activity: Box<dyn PipelineActivitySink>,
}

/// How a dictation that ends before its text is refined ends, with the stages it ran.
type DictationEnd = (DictationOutcome, Vec<StageTiming>);

/// Command words said in one breath with a dictation, so they may reach its transcript.
#[derive(Debug, Default, Clone, Copy)]
struct CommandWords {
    /// The command that started the dictation, said in one breath with its first words.
    opening: Option<VoiceCommand>,
    /// A command said without a pause ended the dictation.
    closing: bool,
}

impl Pipeline {
    pub fn new(
        recorder: Box<dyn AudioRecorder>,
        transcriber: Box<dyn Transcriber>,
        refiner: Box<dyn TextRefiner>,
        paster: Box<dyn TextPaster>,
        sound_player: Box<dyn SoundPlayer>,
        visual_indicator: Box<dyn VisualIndicator>,
        cfg: PipelineConfig,
    ) -> Self {
        Self {
            recorder,
            transcriber: Some(transcriber),
            refiner,
            paster,
            sound_player,
            visual_indicator,
            cfg,
            state: PipelineState::Idle,
            live_session: None,
            dropped_worker: None,
            focus_probe: None,
            focus_wait: FocusWaitCfg::OFF,
            inputs: None,
            pending: VecDeque::new(),
            voice: None,
            command_words: CommandWords::default(),
            shows_refinement: true,
            pending_notice: None,
            archive: None,
            trace: None,
            activity: Box::new(NoActivitySink),
        }
    }

    /// Publishes the activity to `sink` after every input, so a restart can wait for Idle.
    pub fn with_activity_sink(mut self, sink: Box<dyn PipelineActivitySink>) -> Self {
        self.activity = sink;
        self
    }

    /// Text refinement is off: the overlay never shows its processing stage.
    pub fn without_text_refinement_stage(mut self) -> Self {
        self.shows_refinement = false;
        self
    }

    /// Waits for text input focus before a dictation is pasted (see `insert_text`).
    pub fn with_focus_wait(mut self, probe: Box<dyn FocusProbe>, cfg: FocusWaitCfg) -> Self {
        self.focus_probe = Some(probe);
        self.focus_wait = cfg;
        self
    }

    pub fn run(&mut self, rx: mpsc::Receiver<PipelineInput>) -> anyhow::Result<()> {
        // kept in `self` so a focus wait inside `step` sees a hotkey press
        self.inputs = Some(rx);
        if self.voice.is_some() {
            self.turn_listening_on();
        }
        loop {
            let input = match self.pending.pop_front() {
                Some(input) => input,
                None => self
                    .inputs
                    .as_ref()
                    .expect("inputs set above")
                    .recv()
                    .map_err(|_| anyhow::anyhow!("pipeline input channel closed"))?,
            };
            match input {
                PipelineInput::Shutdown => return self.shut_down(),
                PipelineInput::PermissionRevoked(permission) => {
                    return self.shut_down_revoked(permission)
                }
                other => self.handle(other)?,
            }
        }
    }

    /// Handles one input and publishes the activity it leaves the pipeline in; a dictation
    /// being stopped counts as processing until its paste.
    fn handle(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        if self.is_recording() {
            self.activity.publish(PipelineActivity::Processing);
        }
        let outcome = self.dispatch(input);
        self.publish_activity();
        outcome
    }

    fn dispatch(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        if self.voice.is_some() {
            return self.handle_with_voice_control(input);
        }
        match input {
            PipelineInput::Hotkey(signal) => self.step(signal),
            PipelineInput::Shutdown => self.shut_down(),
            PipelineInput::PermissionRevoked(permission) => self.shut_down_revoked(permission),
            other => {
                log::warn!("ignoring {other:?}: voice control is off");
                Ok(())
            }
        }
    }

    fn publish_activity(&self) {
        self.activity.publish(if self.is_recording() {
            PipelineActivity::Recording
        } else {
            PipelineActivity::Idle
        });
    }

    /// Ends this pipeline: a recording in progress is cancelled, listening stops.
    fn shut_down(&mut self) -> anyhow::Result<()> {
        log::info!("pipeline shutting down");
        if self.is_recording() {
            self.cancel_recording()?;
        }
        self.turn_listening_off();
        self.publish_activity();
        Ok(())
    }

    /// Ends this pipeline as a shutdown does; a recording in progress ends with a notice
    /// naming the revoked permission.
    fn shut_down_revoked(&mut self, permission: Permission) -> anyhow::Result<()> {
        let was_recording = self.is_recording();
        self.shut_down()?;
        if was_recording {
            self.visual_indicator
                .show_failure_notice(&FailureNotice::PermissionRevoked(permission));
        }
        Ok(())
    }

    fn step(&mut self, signal: PttSignal) -> anyhow::Result<()> {
        use PipelineState::*;
        log::info!("pipeline recv {signal:?} in state {:?}", self.state);
        let stops = matches!(
            (self.state, signal, self.cfg.recording_mode),
            (Recording, PttSignal::Released, RecordingMode::Hold)
                | (Recording, PttSignal::Pressed, RecordingMode::Toggle)
        );
        match (self.state, signal) {
            (Idle, PttSignal::Pressed) => past_recording_not_started(self.start_recording())?,
            (Recording, _) if stops => self.stop_recording()?,
            (Recording, PttSignal::Cancel) => self.cancel_recording()?,
            (s, sig) => log::warn!("ignoring {sig:?} in state {s:?}"),
        }
        Ok(())
    }

    fn start_recording(&mut self) -> anyhow::Result<()> {
        self.start_recording_from(None, StartCue::Start)
    }

    /// Starts a recording; one started by voice begins at `begins_at`, the end of the
    /// command, with the speech heard since then.
    fn start_recording_from(
        &mut self,
        begins_at: Option<Instant>,
        cue: StartCue,
    ) -> anyhow::Result<()> {
        // With voice control the dictation phrases are listened for before the recording
        // starts, so a "stop dictation" said right away is heard.
        self.follow_dictation(true);
        self.command_words = CommandWords::default();
        self.pending_notice = None;
        match cue {
            StartCue::Start => self.sound_player.play_start(),
            StartCue::Restart => self.sound_player.play_restart(),
        }
        if let Err(e) = self.recorder.start() {
            self.follow_dictation(false);
            return Err(self.recording_not_started(e));
        }
        if let Some(since) = begins_at {
            // Taken once the recorder runs, so the heard speech reaches its first sample,
            // and before live chunks, so they count the speech as the recording's start.
            let lead_in = self.heard_since(since);
            self.recorder.begin_with(lead_in);
        }
        self.open_trace(dictation_start(begins_at, self.cfg.recording_mode));
        if self.cfg.chunking.chunk_threshold_secs > 0 {
            if let Some(chunk_rx) = self.recorder.start_live_chunks(self.cfg.chunking)? {
                let target_chunk_secs =
                    self.cfg.chunking.chunk_threshold_secs + self.cfg.chunking.chunk_overlap_secs;
                log::info!(
                    "live chunking: threshold={}s overlap={}s → target chunk duration ≈ {}s",
                    self.cfg.chunking.chunk_threshold_secs,
                    self.cfg.chunking.chunk_overlap_secs,
                    target_chunk_secs,
                );
                let transcriber = self.transcriber_source();
                let (results_tx, results_rx) = mpsc::sync_channel(1);
                let (live_transcript, recording_end) =
                    LiveTranscript::new(self.visual_indicator.live_transcript_view());
                std::thread::Builder::new()
                    .name("live-transcription".into())
                    .spawn(move || {
                        run_live_worker(transcriber, chunk_rx, results_tx, live_transcript)
                    })
                    .expect("failed to spawn live transcription thread");
                self.live_session = Some(LiveTranscriptionSession {
                    results_rx,
                    recording_end,
                });
            }
        }
        self.visual_indicator.show_recording();
        self.state = PipelineState::Recording;
        Ok(())
    }

    /// The transcriber for a new live worker: here, or still with a dropped dictation's worker.
    fn transcriber_source(&mut self) -> TranscriberSource {
        match (self.transcriber.take(), self.dropped_worker.take()) {
            (Some(transcriber), _) => TranscriberSource::Ready(transcriber),
            (None, Some(results)) => TranscriberSource::ReturnedBy(results),
            (None, None) => panic!("transcriber already taken"),
        }
    }

    /// Waits for the transcriber a dropped dictation's worker still holds.
    fn reclaim_transcriber(&mut self) {
        if let Some(results) = self.dropped_worker.take() {
            self.transcriber = TranscriberSource::ReturnedBy(results).take();
        }
    }

    fn stop_recording(&mut self) -> anyhow::Result<()> {
        let t_user_stop = std::time::Instant::now();
        self.follow_dictation(false);
        self.sound_player.play_stop();
        self.visual_indicator.show_recognizing_speech();
        if let Some(session) = self.live_session.take() {
            session.recording_end.signal();
            self.state = PipelineState::Idle;
            let t_drain_start = std::time::Instant::now();
            self.recorder.flush_live_chunks();
            match session.results_rx.recv() {
                Ok(result) => {
                    let drain_ms = t_drain_start.elapsed().as_millis() as u64;
                    let t_stop_start = std::time::Instant::now();
                    let _ = self.recorder.stop();
                    let stop_ms = t_stop_start.elapsed().as_millis() as u64;
                    self.transcriber = Some(result.transcriber);
                    self.note_lost_chunks(result.lost_chunks, result.texts.is_empty());
                    self.trace_chunks(&result.texts, result.lost_chunks);
                    if result.texts.is_empty() {
                        log::info!("live session: no text (drain {drain_ms}ms stop {stop_ms}ms)");
                        self.end_cycle_on_overlay();
                        let stages = timings([("drain", drain_ms), ("stop", stop_ms)]);
                        self.close_trace(no_text_outcome(result.lost_chunks), &stages);
                    } else {
                        let t_merge_start = std::time::Instant::now();
                        let merged = merge_chunk_texts(result.texts);
                        let merge_ms = t_merge_start.elapsed().as_millis() as u64;
                        log::info!(
                            "live session: {} chars merged (drain {drain_ms}ms stop {stop_ms}ms merge {merge_ms}ms)",
                            merged.len()
                        );
                        match RawTranscript::new(self.without_command_words(&merged)) {
                            Ok(raw) => {
                                self.run_refine_paste(raw, t_user_stop, drain_ms, stop_ms, merge_ms)
                            }
                            Err(e) => {
                                log::warn!("live session: bad transcript: {e}");
                                self.end_cycle_on_overlay();
                                let stages = timings([
                                    ("drain", drain_ms),
                                    ("stop", stop_ms),
                                    ("merge", merge_ms),
                                ]);
                                self.close_trace(DictationOutcome::OnlyCommandWords, &stages);
                            }
                        }
                    }
                }
                Err(_) => {
                    log::error!("live transcription worker channel closed unexpectedly");
                    let _ = self.recorder.stop();
                    self.end_cycle_on_overlay();
                    self.close_trace(DictationOutcome::SpeechRecognitionFailed, &[]);
                }
            }
        } else {
            let t_stop_start = std::time::Instant::now();
            let audio = self.recorder.stop()?;
            let stop_ms = t_stop_start.elapsed().as_millis() as u64;
            self.state = PipelineState::Idle;
            if audio.is_empty() || audio.duration_ms() < self.cfg.min_press_ms {
                log::info!("dropped: too short ({} ms)", audio.duration_ms());
                self.drop_trace();
                self.end_cycle_on_overlay();
                return Ok(());
            }
            self.run_transcribe_refine_paste(audio, t_user_stop, stop_ms);
        }
        Ok(())
    }

    fn cancel_recording(&mut self) -> anyhow::Result<()> {
        self.discard_recording()?;
        self.sound_player.play_cancel();
        self.visual_indicator.hide();
        log::info!("recording cancelled");
        Ok(())
    }

    /// Stops the recording and drops it at once, its last chunk unheard, with no cue: the
    /// worker still transcribing it hands the transcriber to the next live worker.
    fn drop_recording(&mut self) -> anyhow::Result<()> {
        self.follow_dictation(false);
        self.drop_trace();
        self.command_words = CommandWords::default();
        match self.live_session.take() {
            Some(session) => {
                session.recording_end.signal();
                let _ = self.recorder.stop();
                self.dropped_worker = Some(session.results_rx);
            }
            None => {
                self.recorder.stop()?;
            }
        }
        self.state = PipelineState::Idle;
        Ok(())
    }

    /// Stops the recording and drops it, with no cue: the caller gives its own.
    fn discard_recording(&mut self) -> anyhow::Result<()> {
        self.follow_dictation(false);
        self.drop_trace();
        self.command_words = CommandWords::default();
        if let Some(session) = self.live_session.take() {
            session.recording_end.signal();
            self.recorder.flush_live_chunks();
            let _ = self.recorder.stop();
            if let Ok(result) = session.results_rx.recv() {
                self.transcriber = Some(result.transcriber);
            } else {
                log::error!("live worker channel closed on cancel");
            }
        } else {
            let _ = self.recorder.stop()?;
        }
        self.state = PipelineState::Idle;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        matches!(self.state, PipelineState::Recording)
    }

    pub fn run_once(&mut self, audio: AudioBuffer16k) -> anyhow::Result<()> {
        self.reclaim_transcriber();
        self.open_trace(DictationStart::RecordingFile);
        let chunks = split_into_chunks(&audio, self.cfg.chunking);
        if chunks.len() > 1 {
            log::info!("file mode: {} chunks", chunks.len());
        }
        self.trace_audio(audio);
        let t_transcribe = Instant::now();
        let texts = match self.transcribe_chunks(&chunks) {
            Ok(texts) => texts,
            Err(e) => {
                self.close_trace(DictationOutcome::SpeechRecognitionFailed, &[]);
                return Err(e);
            }
        };
        let transcribe_ms = t_transcribe.elapsed().as_millis() as u64;
        self.trace_chunks(&texts, 0);
        let stages = timings([("transcribe", transcribe_ms)]);
        let Ok(raw) = RawTranscript::new(merge_chunk_texts(texts)) else {
            self.close_trace(DictationOutcome::NoText, &stages);
            return Ok(());
        };
        let Some(cleaned) = self.refine(&raw) else {
            self.close_trace(DictationOutcome::NoText, &stages);
            return Ok(());
        };
        let pasted = self.paster.paste(&cleaned);
        let outcome = match pasted {
            Ok(()) => DictationOutcome::Pasted,
            Err(_) => DictationOutcome::PasteFailed,
        };
        self.close_trace(outcome, &stages);
        pasted?;
        eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str());
        Ok(())
    }

    /// The text speech recognition returned for each chunk that had any.
    fn transcribe_chunks(&mut self, chunks: &[AudioBuffer16k]) -> anyhow::Result<Vec<String>> {
        let transcriber = self
            .transcriber
            .as_mut()
            .expect("transcriber not available");
        let mut texts = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            if let Some(raw) = transcriber.transcribe(chunk)? {
                texts.push(raw.as_str().to_owned());
            }
        }
        Ok(texts)
    }

    fn deliver(&mut self, cleaned: &CleanedText) -> DictationOutcome {
        let inputs = self.inputs.as_ref();
        let pending = &mut self.pending;
        let mut hotkey_pressed =
            || inputs.is_some_and(|rx| hotkey_pressed_during_wait(rx, pending));
        let insertion = insert_text(
            cleaned,
            self.paster.as_mut(),
            self.focus_probe.as_deref(),
            self.visual_indicator.as_ref(),
            self.focus_wait,
            &mut hotkey_pressed,
        );
        match &insertion {
            Ok(Insertion::Pasted | Insertion::PastedAfterWait) => {
                eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str())
            }
            Ok(Insertion::LeftInClipboard(_)) => {}
            Err(e) => {
                log::error!("paste failed, dropping cycle: {e:#}");
                self.note_failure(not_pasted(e));
            }
        }
        insertion_outcome(&insertion)
    }

    fn run_transcribe_refine_paste(
        &mut self,
        audio: AudioBuffer16k,
        t_user_stop: std::time::Instant,
        stop_ms: u64,
    ) {
        let duration_s = audio.duration_ms() as f32 / 1000.0;
        let (raw, transcribe_ms) = match self.transcribe_recording(audio, stop_ms) {
            Ok(transcribed) => transcribed,
            Err((outcome, stages)) => {
                self.state = PipelineState::Idle;
                self.end_cycle_on_overlay();
                self.close_trace(outcome, &stages);
                return;
            }
        };

        self.state = PipelineState::Refining;
        log::info!("refining…");
        if self.shows_refinement {
            self.visual_indicator.show_refining_text();
        }
        let t_refine = std::time::Instant::now();
        let Some(cleaned) = self.refine(&raw) else {
            self.state = PipelineState::Idle;
            self.end_cycle_on_overlay();
            let stages = timings([("stop", stop_ms), ("transcribe", transcribe_ms)]);
            self.close_trace(DictationOutcome::NoText, &stages);
            return;
        };
        let refine_ms = t_refine.elapsed().as_millis() as u64;

        self.state = PipelineState::Pasting;
        log::info!("pasting… ({duration_s:.1}s)");
        let t_paste = std::time::Instant::now();
        let outcome = self.deliver(&cleaned);
        let paste_ms = t_paste.elapsed().as_millis() as u64;
        let total_ms = t_user_stop.elapsed().as_millis() as u64;
        self.state = PipelineState::Idle;
        self.end_delivery_on_overlay(outcome);
        log::info!(
            "stop→paste: stop={stop_ms}ms transcribe={transcribe_ms}ms refine={refine_ms}ms paste={paste_ms}ms total={total_ms}ms"
        );
        let stages = timings([
            ("stop", stop_ms),
            ("transcribe", transcribe_ms),
            ("refine", refine_ms),
            ("paste", paste_ms),
            ("total", total_ms),
        ]);
        self.close_trace(outcome, &stages);
    }

    /// The transcript to refine and how long speech recognition took, or how the dictation
    /// ends here.
    fn transcribe_recording(
        &mut self,
        audio: AudioBuffer16k,
        stop_ms: u64,
    ) -> Result<(RawTranscript, u64), DictationEnd> {
        self.state = PipelineState::Transcribing;
        log::info!("transcribing…");
        self.reclaim_transcriber();
        let t_transcribe = std::time::Instant::now();
        let transcribed = self
            .transcriber
            .as_mut()
            .expect("transcriber not available")
            .transcribe(&audio);
        let transcribe_ms = t_transcribe.elapsed().as_millis() as u64;
        self.trace_audio(audio);
        let stages = timings([("stop", stop_ms), ("transcribe", transcribe_ms)]);
        let raw = match transcribed {
            Err(e) => {
                log::warn!("transcription failed, dropping cycle: {e:#}");
                self.note_failure(FailureNotice::DictationLost(FailingPart::SpeechRecognition));
                return Err((DictationOutcome::SpeechRecognitionFailed, stages));
            }
            Ok(None) => return Err((DictationOutcome::NoText, stages)),
            Ok(Some(r)) => r,
        };
        self.trace_chunks(&[raw.as_str().to_owned()], 0);
        let Ok(raw) = RawTranscript::new(self.without_command_words(raw.as_str())) else {
            log::info!("dropped: only command words were transcribed");
            return Err((DictationOutcome::OnlyCommandWords, stages));
        };
        Ok((raw, transcribe_ms))
    }

    /// The text to paste; a fallback to the raw transcript is noted for the notice.
    fn refine(&mut self, raw: &RawTranscript) -> Option<CleanedText> {
        let refined = refine_or_raw(self.refiner.as_mut(), raw);
        self.trace_refinement(raw, refined.as_ref().map(|r| (&r.text, r.fell_back)));
        let refined = refined?;
        if refined.fell_back {
            self.note_failure(FailureNotice::PastedUnrefined(FailingPart::TextRefinement));
        }
        Some(refined.text)
    }

    /// Notes the chunks speech recognition lost: the whole dictation when none had text.
    fn note_lost_chunks(&mut self, lost_chunks: usize, no_text: bool) {
        if lost_chunks == 0 {
            return;
        }
        let part = FailingPart::SpeechRecognition;
        self.note_failure(if no_text {
            FailureNotice::DictationLost(part)
        } else {
            FailureNotice::PartOfDictationLost(part)
        });
    }

    /// `text` without the command words said in one breath with the dictation.
    fn without_command_words(&mut self, text: &str) -> String {
        let words = std::mem::take(&mut self.command_words);
        let text = match words.opening {
            Some(command) => without_leading_command(command, text),
            None => text.to_owned(),
        };
        if words.closing {
            without_trailing_command(&text)
        } else {
            text
        }
    }

    fn run_refine_paste(
        &mut self,
        raw: RawTranscript,
        t_user_stop: std::time::Instant,
        drain_ms: u64,
        stop_ms: u64,
        merge_ms: u64,
    ) {
        self.state = PipelineState::Refining;
        log::info!("refining…");
        if self.shows_refinement {
            self.visual_indicator.show_refining_text();
        }
        let t_refine = std::time::Instant::now();
        let Some(cleaned) = self.refine(&raw) else {
            self.state = PipelineState::Idle;
            self.end_cycle_on_overlay();
            let stages = timings([("drain", drain_ms), ("stop", stop_ms), ("merge", merge_ms)]);
            self.close_trace(DictationOutcome::NoText, &stages);
            return;
        };
        let refine_ms = t_refine.elapsed().as_millis() as u64;
        self.state = PipelineState::Pasting;
        log::info!("pasting… (live)");
        let t_paste = std::time::Instant::now();
        let outcome = self.deliver(&cleaned);
        let paste_ms = t_paste.elapsed().as_millis() as u64;
        let total_ms = t_user_stop.elapsed().as_millis() as u64;
        self.state = PipelineState::Idle;
        self.end_delivery_on_overlay(outcome);
        log::info!(
            "stop→paste: drain={drain_ms}ms stop={stop_ms}ms merge={merge_ms}ms refine={refine_ms}ms paste={paste_ms}ms total={total_ms}ms"
        );
        let stages = timings([
            ("drain", drain_ms),
            ("stop", stop_ms),
            ("merge", merge_ms),
            ("refine", refine_ms),
            ("paste", paste_ms),
            ("total", total_ms),
        ]);
        self.close_trace(outcome, &stages);
    }
}

fn run_live_worker(
    transcriber: TranscriberSource,
    chunk_rx: mpsc::Receiver<LiveChunk>,
    results_tx: mpsc::SyncSender<LiveSessionResult>,
    live_transcript: LiveTranscript,
) {
    let Some(mut transcriber) = transcriber.take() else {
        log::error!("live worker: the dropped dictation's worker never returned the transcriber");
        return;
    };
    let mut texts = Vec::new();
    let mut lost_chunks = 0usize;
    let mut chunk_num = 0usize;
    while let Ok(chunk) = chunk_rx.recv() {
        let t_recv = std::time::Instant::now();
        let t_queued_ms = t_recv
            .saturating_duration_since(chunk.enqueued_at)
            .as_millis();
        let audio = chunk.audio;
        chunk_num += 1;
        let duration_secs = audio.duration_ms() as f32 / 1000.0;
        let t0 = std::time::Instant::now();
        match transcriber.transcribe(&audio) {
            Ok(Some(raw)) => {
                log::info!(
                    "chunk {} | {:.1}s audio | queued {}ms | transcribed in {:.2}s",
                    chunk_num,
                    duration_secs,
                    t_queued_ms,
                    t0.elapsed().as_secs_f32(),
                );
                texts.push(raw.as_str().to_owned());
                live_transcript.show(&texts);
            }
            Ok(None) => {
                log::info!(
                    "chunk {chunk_num} | {duration_secs:.1}s audio | queued {t_queued_ms}ms | hallucination dropped in {:.2}s",
                    t0.elapsed().as_secs_f32(),
                );
            }
            Err(e) => {
                lost_chunks += 1;
                log::error!(
                    "chunk {chunk_num} lost | {duration_secs:.1}s audio not transcribed, the rest of the dictation is kept: {e:#}"
                );
            }
        }
    }
    log::info!("live worker: {chunk_num} chunks done, returning transcriber");
    let _ = results_tx.send(LiveSessionResult {
        texts,
        lost_chunks,
        transcriber,
    });
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc, Mutex};

    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::chunking::ChunkingCfg;
    use crate::core::application::live_transcript_view::{
        HiddenLiveTranscript, MockLiveTranscriptView,
    };
    use crate::core::application::pipeline_activity::{PipelineActivity, PipelineActivitySink};
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::text_refiner::MockTextRefiner;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::{CleanedText, RawTranscript};
    use anyhow::anyhow;

    /// Runs the live worker over chunks the transcriber answers with `answers` in turn.
    fn run_worker_over(answers: Vec<Option<&'static str>>, view: MockLiveTranscriptView) {
        let mut transcriber = MockTranscriber::new();
        let mut answers = answers.into_iter();
        transcriber.expect_transcribe().returning(move |_| {
            Ok(answers
                .next()
                .flatten()
                .map(|t| RawTranscript::new(t.to_owned()).unwrap()))
        });
        let (chunk_tx, chunk_rx) = mpsc::channel();
        for _ in 0..2 {
            chunk_tx
                .send(LiveChunk {
                    audio: one_second_audio(),
                    enqueued_at: Instant::now(),
                })
                .unwrap();
        }
        drop(chunk_tx);
        let (results_tx, _results_rx) = mpsc::sync_channel(1);
        let (live_transcript, _recording_end) = LiveTranscript::new(Box::new(view));
        let transcriber = TranscriberSource::Ready(Box::new(transcriber));
        run_live_worker(transcriber, chunk_rx, results_tx, live_transcript);
    }

    #[test]
    fn should_show_the_stitched_text_after_every_chunk_with_text() {
        // Given
        let mut view = MockLiveTranscriptView::new();
        let mut seq = mockall::Sequence::new();
        view.expect_show_live_transcript()
            .withf(|text| text == "one two three")
            .once()
            .in_sequence(&mut seq)
            .return_const(());
        view.expect_show_live_transcript()
            .withf(|text| text == "one two three four five")
            .once()
            .in_sequence(&mut seq)
            .return_const(());
        // When / Then
        run_worker_over(
            vec![Some("one two three"), Some("two three four five")],
            view,
        );
    }

    #[test]
    fn should_show_nothing_new_for_a_chunk_without_text() {
        // Given
        let mut view = MockLiveTranscriptView::new();
        view.expect_show_live_transcript()
            .withf(|text| text == "one two three")
            .once()
            .return_const(());
        // When / Then
        run_worker_over(vec![Some("one two three"), None], view);
    }

    fn one_second_audio() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![0i16; 16_000])
    }

    fn noop_sound_player() -> MockSoundPlayer {
        let mut p = MockSoundPlayer::new();
        p.expect_play_start().times(..).return_const(());
        p.expect_play_stop().times(..).return_const(());
        p.expect_play_cancel().times(..).return_const(());
        p
    }

    fn noop_visual_indicator() -> MockVisualIndicator {
        let mut v = MockVisualIndicator::new();
        v.expect_show_recording().times(..).return_const(());
        v.expect_show_recognizing_speech()
            .times(..)
            .return_const(());
        v.expect_show_refining_text().times(..).return_const(());
        v.expect_show_pasted().times(..).return_const(());
        v.expect_hide().times(..).return_const(());
        v.expect_show_failure_notice().times(..).return_const(());
        v.expect_live_transcript_view()
            .times(..)
            .returning(|| Box::new(HiddenLiveTranscript));
        v
    }

    fn make_recorder_with_start_stop(audio: AudioBuffer16k) -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(move || Ok(audio.clone()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        recorder
    }

    fn make_pipeline(
        recorder: MockAudioRecorder,
        transcriber: MockTranscriber,
        refiner: MockTextRefiner,
        paster: MockTextPaster,
    ) -> Pipeline {
        make_pipeline_with_mode(
            recorder,
            transcriber,
            refiner,
            paster,
            RecordingMode::Toggle,
        )
    }

    fn make_pipeline_with_mode(
        recorder: MockAudioRecorder,
        transcriber: MockTranscriber,
        refiner: MockTextRefiner,
        paster: MockTextPaster,
        recording_mode: RecordingMode,
    ) -> Pipeline {
        Pipeline::new(
            Box::new(recorder),
            Box::new(transcriber),
            Box::new(refiner),
            Box::new(paster),
            Box::new(noop_sound_player()),
            Box::new(noop_visual_indicator()),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode,
                chunking: ChunkingCfg::default(),
            },
        )
    }

    struct RecordingActivitySink(Arc<Mutex<Vec<PipelineActivity>>>);

    impl PipelineActivitySink for RecordingActivitySink {
        fn publish(&self, activity: PipelineActivity) {
            self.0.lock().unwrap().push(activity);
        }
    }

    fn observed_activity() -> (Arc<Mutex<Vec<PipelineActivity>>>, RecordingActivitySink) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        (Arc::clone(&seen), RecordingActivitySink(Arc::clone(&seen)))
    }

    fn pasting_pipeline(recorder: MockAudioRecorder) -> Pipeline {
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .returning(|_| Ok(Some(RawTranscript::new("hello world".to_owned()).unwrap())));
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .returning(|_| Ok(CleanedText::new("hello world".to_owned()).unwrap()));
        let mut paster = MockTextPaster::new();
        paster.expect_paste().returning(|_| Ok(()));
        make_pipeline(recorder, transcriber, refiner, paster)
    }

    fn make_recorder_with_start() -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        recorder
    }

    #[test]
    fn should_publish_recording_when_a_recording_starts() {
        // Given
        let (seen, sink) = observed_activity();
        let mut pipeline =
            pasting_pipeline(make_recorder_with_start()).with_activity_sink(Box::new(sink));
        // When
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        // Then
        assert_eq!(*seen.lock().unwrap(), vec![PipelineActivity::Recording]);
    }

    #[test]
    fn should_publish_processing_through_the_paste_then_idle() {
        // Given
        let (seen, sink) = observed_activity();
        let mut pipeline = pasting_pipeline(make_recorder_with_start_stop(one_second_audio()))
            .with_activity_sink(Box::new(sink));
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        // Then
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                PipelineActivity::Recording,
                PipelineActivity::Processing,
                PipelineActivity::Idle
            ]
        );
    }

    #[test]
    fn should_publish_idle_after_a_cancel() {
        // Given
        let (seen, sink) = observed_activity();
        let mut pipeline = pasting_pipeline(make_recorder_with_start_stop(one_second_audio()))
            .with_activity_sink(Box::new(sink));
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        // When
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Cancel))
            .unwrap();
        // Then
        assert_eq!(seen.lock().unwrap().last(), Some(&PipelineActivity::Idle));
    }

    #[test]
    fn should_return_from_run_on_shutdown_while_idle() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().never();
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        let (inputs, rx) = mpsc::channel::<PipelineInput>();
        inputs.send(PipelineInput::Shutdown).unwrap();
        // When
        let result = pipeline.run(rx);
        // Then — the channel is still open, so only the shutdown ended the loop
        assert!(result.is_ok());
        drop(inputs);
    }

    #[test]
    fn should_cancel_the_recording_with_a_notice_naming_a_revoked_permission() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));
        let mut sounds = MockSoundPlayer::new();
        sounds.expect_play_start().once().return_const(());
        sounds.expect_play_cancel().once().return_const(());
        let mut indicator = MockVisualIndicator::new();
        indicator.expect_show_recording().return_const(());
        indicator.expect_hide().times(..).return_const(());
        indicator
            .expect_show_failure_notice()
            .withf(|notice| *notice == FailureNotice::PermissionRevoked(Permission::Microphone))
            .once()
            .return_const(());
        indicator
            .expect_live_transcript_view()
            .times(..)
            .returning(|| Box::new(HiddenLiveTranscript));
        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(sounds),
            Box::new(indicator),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        let (inputs, rx) = mpsc::channel::<PipelineInput>();
        inputs
            .send(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        inputs
            .send(PipelineInput::PermissionRevoked(Permission::Microphone))
            .unwrap();
        // When
        let result = pipeline.run(rx);
        // Then
        assert!(result.is_ok());
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_cancel_the_recording_on_shutdown() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));
        let mut sounds = MockSoundPlayer::new();
        sounds.expect_play_start().once().return_const(());
        sounds.expect_play_cancel().once().return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(paster),
            Box::new(sounds),
            Box::new(noop_visual_indicator()),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        let (inputs, rx) = mpsc::channel::<PipelineInput>();
        inputs
            .send(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        inputs.send(PipelineInput::Shutdown).unwrap();
        // When
        let result = pipeline.run(rx);
        // Then
        assert!(result.is_ok());
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_start_in_idle_state() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        // (no signals sent)
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_transition_idle_to_recording_on_pressed() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Recording));
    }

    #[test]
    fn should_drop_cycle_when_audio_too_short() {
        // Given
        let recorder = make_recorder_with_start_stop(AudioBuffer16k::new(vec![0i16; 10]));
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_complete_full_pipeline_and_return_to_idle() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello world".to_owned()).unwrap())));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello world".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_drop_cycle_and_not_paste_when_transcription_fails() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Err(anyhow!("whisper failure")));

        let mut pipeline = make_pipeline(
            recorder,
            transcriber,
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_paste_raw_transcript_when_refinement_fails() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Err(anyhow!("llm failure")));

        let mut paster = MockTextPaster::new();
        paster
            .expect_paste()
            .withf(|text| text.as_str() == "hello")
            .once()
            .returning(|_| Ok(()));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_paste_raw_transcript_in_file_mode_when_refinement_fails() {
        // Given
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Err(anyhow!("text refinement stopped at its output limit")));
        let mut paster = MockTextPaster::new();
        paster
            .expect_paste()
            .withf(|text| text.as_str() == "hello")
            .once()
            .returning(|_| Ok(()));
        let mut pipeline = make_pipeline(MockAudioRecorder::new(), transcriber, refiner, paster);
        // When
        let result = pipeline.run_once(one_second_audio());
        // Then
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn should_drop_cycle_when_paste_fails() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster
            .expect_paste()
            .once()
            .returning(|_| Err(anyhow!("accessibility denied")));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_stop_recording_on_released_in_hold_mode() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut pipeline =
            make_pipeline_with_mode(recorder, transcriber, refiner, paster, RecordingMode::Hold);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_warn_and_stay_idle_on_released_while_idle() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_run_loop_until_channel_closed() {
        // Given
        let (tx, rx) = mpsc::channel::<PipelineInput>();
        drop(tx);

        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        let result = pipeline.run(rx);
        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("channel closed"));
    }

    #[test]
    fn should_ignore_pressed_signal_while_transcribing() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.state = PipelineState::Transcribing;
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Transcribing));
    }

    #[test]
    fn should_ignore_released_signal_while_pasting() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.state = PipelineState::Pasting;
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Pasting));
    }

    #[test]
    fn should_cancel_recording_and_return_to_idle_on_cancel_signal() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — no transcription, back to Idle
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_call_hide_when_recording_cancelled() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_recognizing_speech().never();
        visual.expect_show_refining_text().never();
        visual.expect_hide().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — no processing stage shown, hide called once
    }

    #[test]
    fn should_play_cancel_sound_when_recording_cancelled() {
        // Given
        let recorder = make_recorder_with_start_stop(one_second_audio());

        let mut sound = MockSoundPlayer::new();
        sound.expect_play_start().once().return_const(());
        sound.expect_play_stop().never();
        sound.expect_play_cancel().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(sound),
            Box::new(noop_visual_indicator()),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — play_cancel called once, play_stop never called
    }

    #[test]
    fn should_ignore_cancel_signal_while_idle() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_run_once_in_chunks_when_the_recording_is_longer_than_the_threshold() {
        // Given — 70 s of sound and a 30 s chunk threshold: chunks end at 30 s, 60 s, 70 s
        let mut texts = ["one two three", "three four five", "five six"].into_iter();
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .times(3)
            .returning(move |_| Ok(Some(RawTranscript::new(texts.next().unwrap().to_owned())?)));
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|raw| Ok(CleanedText::new(raw.as_str().to_owned()).unwrap()));
        let mut paster = MockTextPaster::new();
        paster
            .expect_paste()
            .once()
            .withf(|text| text.as_str() == "one two three four five six")
            .returning(|_| Ok(()));
        let mut pipeline = make_pipeline(MockAudioRecorder::new(), transcriber, refiner, paster);
        pipeline.cfg.chunking.chunk_threshold_secs = 30;
        // When
        pipeline
            .run_once(AudioBuffer16k::new(vec![10_000i16; 70 * 16_000]))
            .unwrap();
        // Then — three transcriptions (checked by the mock) and one merged paste
    }

    #[test]
    fn should_run_once_and_return_ok_for_successful_pipeline() {
        // Given
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        // When
        let result = pipeline.run_once(one_second_audio());
        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_run_once_and_return_err_when_transcription_fails() {
        // Given
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Err(anyhow::anyhow!("whisper failure")));

        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        let mut pipeline = make_pipeline(
            recorder,
            transcriber,
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        let result = pipeline.run_once(one_second_audio());
        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_call_show_recording_when_transitioning_to_recording() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual
            .expect_show_recognizing_speech()
            .times(..)
            .return_const(());
        visual
            .expect_show_refining_text()
            .times(..)
            .return_const(());
        visual.expect_show_pasted().times(..).return_const(());
        visual.expect_hide().times(..).return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — MockVisualIndicator verifies exactly once show_recording was called
    }

    /// What the overlay was asked to show for a dictation of "hello", whose refinement
    /// answers `refined`.
    fn overlay_signals(
        refinement_shown: bool,
        refined: fn() -> anyhow::Result<CleanedText>,
    ) -> Vec<String> {
        let recorder = make_recorder_with_start_stop(one_second_audio());
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(Some(RawTranscript::new("hello".to_owned()).unwrap())));
        let mut refiner = MockTextRefiner::new();
        refiner.expect_refine().once().returning(move |_| refined());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        let (visual, shown) = noting_visual_indicator();
        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(transcriber),
            Box::new(refiner),
            Box::new(paster),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        if !refinement_shown {
            pipeline = pipeline.without_text_refinement_stage();
        }
        pipeline.step(PttSignal::Pressed).unwrap();
        pipeline.step(PttSignal::Pressed).unwrap();
        let shown = shown.lock().unwrap().clone();
        shown
    }

    /// An overlay that notes each signal.
    fn noting_visual_indicator() -> (MockVisualIndicator, Arc<Mutex<Vec<String>>>) {
        let shown = Arc::new(Mutex::new(Vec::new()));
        let mut visual = MockVisualIndicator::new();
        let note = |signal: &'static str| {
            let shown = Arc::clone(&shown);
            move || shown.lock().unwrap().push(signal.to_owned())
        };
        visual.expect_show_recording().returning(note("recording"));
        visual
            .expect_show_recognizing_speech()
            .returning(note("recognizing speech"));
        visual
            .expect_show_refining_text()
            .returning(note("refining text"));
        visual.expect_show_pasted().returning(note("pasted"));
        let noted = Arc::clone(&shown);
        visual
            .expect_show_failure_notice()
            .returning(move |notice| noted.lock().unwrap().push(format!("{notice:?}")));
        visual.expect_hide().returning(note("hidden"));
        (visual, shown)
    }

    fn refined_hello() -> anyhow::Result<CleanedText> {
        Ok(CleanedText::new("Hello.".to_owned())?)
    }

    fn refinement_down() -> anyhow::Result<CleanedText> {
        Err(anyhow!("llm failure"))
    }

    #[test]
    fn should_show_speech_recognition_then_text_refinement_then_the_pasted_sign() {
        assert_eq!(
            overlay_signals(true, refined_hello),
            ["recording", "recognizing speech", "refining text", "pasted"]
        );
    }

    #[test]
    fn should_not_show_text_refinement_when_its_stage_is_off() {
        assert_eq!(
            overlay_signals(false, refined_hello),
            ["recording", "recognizing speech", "pasted"]
        );
    }

    #[test]
    fn should_end_on_the_pending_failure_notice_instead_of_the_pasted_sign() {
        let notice = FailureNotice::PastedUnrefined(FailingPart::TextRefinement);
        assert_eq!(
            overlay_signals(true, refinement_down),
            [
                "recording".to_owned(),
                "recognizing speech".to_owned(),
                "refining text".to_owned(),
                format!("{notice:?}"),
            ]
        );
    }

    #[test]
    fn should_call_hide_when_audio_too_short() {
        // Given
        let recorder = make_recorder_with_start_stop(AudioBuffer16k::new(vec![0i16; 10]));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual
            .expect_show_recognizing_speech()
            .once()
            .return_const(());
        visual.expect_hide().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg::default(),
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — speech recognition shows at once on stop, hide fires after too-short check
    }
}
