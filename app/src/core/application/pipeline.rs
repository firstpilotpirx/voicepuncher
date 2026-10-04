use std::sync::mpsc;

use crate::core::application::audio_recorder::LiveChunk;
use crate::core::application::chunking::{merge_chunk_texts, split_into_chunks, ChunkingCfg};
use crate::core::application::{
    AudioRecorder, SoundPlayer, TextPaster, TextRefiner, Transcriber, VisualIndicator,
};
use crate::core::domain::{AudioBuffer16k, CleanedText, PttSignal, RawTranscript, RecordingMode};

#[derive(Debug, Clone, Copy)]
pub struct PipelineConfig {
    pub min_press_ms: u64,
    pub recording_mode: RecordingMode,
    pub chunking: ChunkingCfg,
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
}

struct LiveSessionResult {
    texts: Vec<String>,
    transcriber: Box<dyn Transcriber>,
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
        }
    }

    pub fn run(&mut self, rx: mpsc::Receiver<PttSignal>) -> anyhow::Result<()> {
        loop {
            let signal = rx
                .recv()
                .map_err(|_| anyhow::anyhow!("hotkey listener channel closed"))?;
            self.step(signal)?;
        }
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
            (Idle, PttSignal::Pressed) => {
                self.sound_player.play_start();
                self.recorder.start()?;
                if self.cfg.chunking.chunk_threshold_secs > 0 {
                    if let Some(chunk_rx) = self.recorder.start_live_chunks(self.cfg.chunking)? {
                        let target_chunk_secs = self.cfg.chunking.chunk_threshold_secs
                            + self.cfg.chunking.chunk_overlap_secs;
                        log::info!(
                            "live chunking: threshold={}s overlap={}s → target chunk duration ≈ {}s",
                            self.cfg.chunking.chunk_threshold_secs,
                            self.cfg.chunking.chunk_overlap_secs,
                            target_chunk_secs,
                        );
                        let transcriber =
                            self.transcriber.take().expect("transcriber already taken");
                        let (results_tx, results_rx) = mpsc::sync_channel(1);
                        std::thread::Builder::new()
                            .name("live-transcription".into())
                            .spawn(move || run_live_worker(transcriber, chunk_rx, results_tx))
                            .expect("failed to spawn live transcription thread");
                        self.live_session = Some(LiveTranscriptionSession { results_rx });
                    }
                }
                self.visual_indicator.show_recording();
                self.state = Recording;
            }
            (Recording, _) if stops => {
                let t_user_stop = std::time::Instant::now();
                self.sound_player.play_stop();
                self.visual_indicator.show_processing();
                if let Some(session) = self.live_session.take() {
                    self.state = Idle;
                    let t_drain_start = std::time::Instant::now();
                    self.recorder.flush_live_chunks();
                    match session.results_rx.recv() {
                        Ok(result) => {
                            let drain_ms = t_drain_start.elapsed().as_millis() as u64;
                            let t_stop_start = std::time::Instant::now();
                            let _ = self.recorder.stop();
                            let stop_ms = t_stop_start.elapsed().as_millis() as u64;
                            self.transcriber = Some(result.transcriber);
                            if result.texts.is_empty() {
                                log::info!(
                                    "live session: no text (drain {drain_ms}ms stop {stop_ms}ms)"
                                );
                                self.visual_indicator.hide();
                            } else {
                                let t_merge_start = std::time::Instant::now();
                                let merged = merge_chunk_texts(result.texts);
                                let merge_ms = t_merge_start.elapsed().as_millis() as u64;
                                log::info!(
                                    "live session: {} chars merged (drain {drain_ms}ms stop {stop_ms}ms merge {merge_ms}ms)",
                                    merged.len()
                                );
                                match RawTranscript::new(merged) {
                                    Ok(raw) => self.run_refine_paste(
                                        raw,
                                        t_user_stop,
                                        drain_ms,
                                        stop_ms,
                                        merge_ms,
                                    ),
                                    Err(e) => {
                                        log::warn!("live session: bad transcript: {e}");
                                        self.visual_indicator.hide();
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            log::error!("live transcription worker channel closed unexpectedly");
                            let _ = self.recorder.stop();
                            self.visual_indicator.hide();
                        }
                    }
                } else {
                    let t_stop_start = std::time::Instant::now();
                    let audio = self.recorder.stop()?;
                    let stop_ms = t_stop_start.elapsed().as_millis() as u64;
                    self.state = Idle;
                    if audio.is_empty() || audio.duration_ms() < self.cfg.min_press_ms {
                        log::info!("dropped: too short ({} ms)", audio.duration_ms());
                        self.visual_indicator.hide();
                        return Ok(());
                    }
                    self.run_transcribe_refine_paste(audio, t_user_stop, stop_ms);
                }
            }
            (Recording, PttSignal::Cancel) => {
                if let Some(session) = self.live_session.take() {
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
                self.sound_player.play_cancel();
                self.visual_indicator.hide();
                self.state = Idle;
                log::info!("recording cancelled");
            }
            (s, sig) => log::warn!("ignoring {sig:?} in state {s:?}"),
        }
        Ok(())
    }

    pub fn run_once(&mut self, audio: AudioBuffer16k) -> anyhow::Result<()> {
        let chunks = split_into_chunks(&audio, self.cfg.chunking);
        if chunks.len() > 1 {
            log::info!("file mode: {} chunks", chunks.len());
        }
        let transcriber = self
            .transcriber
            .as_mut()
            .expect("transcriber not available");
        let mut texts = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            if let Some(raw) = transcriber.transcribe(chunk)? {
                texts.push(raw.as_str().to_owned());
            }
        }
        if texts.is_empty() {
            return Ok(());
        }
        let raw = RawTranscript::new(merge_chunk_texts(texts))?;
        let Some(cleaned) = self.refine_or_raw(&raw) else {
            return Ok(());
        };
        self.paster.paste(&cleaned)?;
        eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str());
        Ok(())
    }

    /// Refined text, or the raw transcript when refinement fails; `None` only when even
    /// the raw transcript cannot be pasted.
    fn refine_or_raw(&mut self, raw: &RawTranscript) -> Option<CleanedText> {
        match self.refiner.refine(raw) {
            Ok(cleaned) => Some(cleaned),
            Err(e) => {
                log::warn!("refinement failed, falling back to raw transcript: {e:#}");
                CleanedText::new(raw.as_str().to_owned())
                    .inspect_err(|e| {
                        log::error!("raw transcript fallback failed, dropping cycle: {e}")
                    })
                    .ok()
            }
        }
    }

    fn run_transcribe_refine_paste(
        &mut self,
        audio: AudioBuffer16k,
        t_user_stop: std::time::Instant,
        stop_ms: u64,
    ) {
        let duration_s = audio.duration_ms() as f32 / 1000.0;
        self.state = PipelineState::Transcribing;
        log::info!("transcribing…");
        let t_transcribe = std::time::Instant::now();
        let raw = match self
            .transcriber
            .as_mut()
            .expect("transcriber not available")
            .transcribe(&audio)
        {
            Err(e) => {
                log::warn!("transcription failed, dropping cycle: {e:#}");
                self.state = PipelineState::Idle;
                self.visual_indicator.hide();
                return;
            }
            Ok(None) => {
                self.state = PipelineState::Idle;
                self.visual_indicator.hide();
                return;
            }
            Ok(Some(r)) => r,
        };
        let transcribe_ms = t_transcribe.elapsed().as_millis() as u64;

        self.state = PipelineState::Refining;
        log::info!("refining…");
        let t_refine = std::time::Instant::now();
        let Some(cleaned) = self.refine_or_raw(&raw) else {
            self.state = PipelineState::Idle;
            self.visual_indicator.hide();
            return;
        };
        let refine_ms = t_refine.elapsed().as_millis() as u64;

        self.state = PipelineState::Pasting;
        log::info!("pasting… ({duration_s:.1}s)");
        let t_paste = std::time::Instant::now();
        match self.paster.paste(&cleaned) {
            Ok(()) => eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str()),
            Err(e) => log::error!("paste failed, dropping cycle: {e:#}"),
        }
        let paste_ms = t_paste.elapsed().as_millis() as u64;
        let total_ms = t_user_stop.elapsed().as_millis() as u64;
        self.state = PipelineState::Idle;
        self.visual_indicator.hide();
        log::info!(
            "stop→paste: stop={stop_ms}ms transcribe={transcribe_ms}ms refine={refine_ms}ms paste={paste_ms}ms total={total_ms}ms"
        );
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
        let t_refine = std::time::Instant::now();
        let Some(cleaned) = self.refine_or_raw(&raw) else {
            self.state = PipelineState::Idle;
            self.visual_indicator.hide();
            return;
        };
        let refine_ms = t_refine.elapsed().as_millis() as u64;
        self.state = PipelineState::Pasting;
        log::info!("pasting… (live)");
        let t_paste = std::time::Instant::now();
        match self.paster.paste(&cleaned) {
            Ok(()) => eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str()),
            Err(e) => log::error!("paste failed, dropping cycle: {e:#}"),
        }
        let paste_ms = t_paste.elapsed().as_millis() as u64;
        let total_ms = t_user_stop.elapsed().as_millis() as u64;
        self.state = PipelineState::Idle;
        self.visual_indicator.hide();
        log::info!(
            "stop→paste: drain={drain_ms}ms stop={stop_ms}ms merge={merge_ms}ms refine={refine_ms}ms paste={paste_ms}ms total={total_ms}ms"
        );
    }
}

fn run_live_worker(
    mut transcriber: Box<dyn Transcriber>,
    chunk_rx: mpsc::Receiver<LiveChunk>,
    results_tx: mpsc::SyncSender<LiveSessionResult>,
) {
    let mut texts = Vec::new();
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
            }
            Ok(None) => {
                log::info!(
                    "chunk {chunk_num} | {duration_secs:.1}s audio | queued {t_queued_ms}ms | hallucination dropped in {:.2}s",
                    t0.elapsed().as_secs_f32(),
                );
            }
            Err(e) => log::warn!("chunk {chunk_num} failed: {e:#}"),
        }
    }
    log::info!("live worker: {chunk_num} chunks done, returning transcriber");
    let _ = results_tx.send(LiveSessionResult { texts, transcriber });
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::chunking::ChunkingCfg;
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::text_refiner::MockTextRefiner;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::{CleanedText, RawTranscript};
    use anyhow::anyhow;

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
        v.expect_show_processing().times(..).return_const(());
        v.expect_hide().times(..).return_const(());
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
        let (tx, rx) = mpsc::channel::<PttSignal>();
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
        visual.expect_show_processing().never();
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
        // Then — show_processing never called, hide called once
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
        visual.expect_show_processing().times(..).return_const(());
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

    #[test]
    fn should_call_show_processing_when_transcribing_starts() {
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

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().once().return_const(());
        visual.expect_hide().once().return_const(());

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
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — MockVisualIndicator verifies the exact call sequence
    }

    #[test]
    fn should_call_hide_when_audio_too_short() {
        // Given
        let recorder = make_recorder_with_start_stop(AudioBuffer16k::new(vec![0i16; 10]));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().once().return_const(());
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
        // Then — show_processing fires immediately on stop, hide fires after too-short check
    }
}
