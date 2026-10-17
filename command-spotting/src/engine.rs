//! Runs the keyword spotter and the voice activity detector over one connection's audio.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, OnlineStream, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

use crate::confirm::{Hear, Heard, RecentAudio, UtteranceTranscriber};
use crate::keywords::{self, Keyword, UnknownPhrases};
use crate::matcher::PhraseMatcher;
use crate::orderer::{EventOrderer, Segment, Spotted};
use crate::outbox::Outbox;
use crate::protocol::ServerEvent;
use crate::worker::{ConfirmationWorker, JobKind};

mod clip;
mod confirming;
mod pause;

use confirming::JobContext;
use pause::PauseWatch;

pub const SAMPLE_RATE: u32 = 16_000;
const SILERO_WINDOW: usize = 512;
const VAD_BUFFER_SECS: f32 = 30.0;
const VAD_SPEECH_THRESHOLD: f32 = 0.5;
/// Shorter speech is noise to the detector: a short "start" or "enter" lasts about 0.2 s.
const MIN_SPEECH_SECS: f32 = 0.15;
const MAX_SPEECH_SECS: f32 = 20.0;
/// How long the last spotted token lasts after its reported time.
const LAST_TOKEN_SECS: f32 = 0.2;
/// How long before its reported time the first token was spoken: the spotter reports a
/// token once it has decoded it.
const FIRST_TOKEN_SECS: f32 = 0.25;
/// How far behind the audio fed so far the spotter reports a phrase's end.
const DETECTION_LAG_SECS: f32 = 0.25;
/// Longest gap between two tokens of one spoken phrase.
const MAX_TOKEN_GAP_SECS: f32 = 0.3;
/// Audio kept for the speech recognition of an utterance after its segment closed.
const RECENT_AUDIO_SECS: f32 = 12.0;
/// Speech before a segment's start that the voice activity detector cut off: it notices
/// an utterance up to 0.7 s late, most on a soft start like the "st" of "stop".
const PRE_ROLL_SECS: f32 = 0.8;
const POST_ROLL_SECS: f32 = 0.2;
/// 100 ms: the resolution at which a spotted phrase is placed in the stream.
const DECODE_FRAME_SAMPLES: usize = 1_600;
/// How much of an utterance's end is searched for a command that ends it, or for a doubled
/// or chained command while it runs.
const WINDOW_SECS: f32 = 2.5;
/// How often a running utterance is searched for a doubled or chained command.
const WINDOW_EVERY_SECS: f32 = 1.0;
/// How long an utterance runs before the first such search: a shorter one is searched
/// once it ends, which spares the speech recognition service a request.
const FIRST_WINDOW_SECS: f32 = 2.0;
/// How long an utterance runs before its start is searched for "start": long enough for
/// "start" and the first word after it.
const HEAD_CHECK_SECS: f32 = 1.5;
/// How far before the end of its utterance a command heard while it ran may end and still
/// end the utterance: the detector holds speech this much past the last word.
const TAIL_SLACK_SECS: f32 = 0.6;
/// How late the voice activity detector notices speech has started.
const ONSET_SECS: f32 = 0.3;

pub struct EngineCfg {
    pub model_dir: PathBuf,
    pub vad_model: PathBuf,
    pub threads: i32,
    /// The silence that ends a speech segment: what counts as a pause.
    pub min_silence_secs: f32,
    /// Where each speech segment is saved as a WAV file, for tuning on real voices.
    pub save_speech_dir: Option<PathBuf>,
    /// Keyword lines in the format of `keywords.txt`, in place of the built-in ones.
    pub keywords: Option<String>,
    /// The speech recognition service that confirms short utterances; none: keywords only.
    pub transcription_url: Option<String>,
    /// Longest utterance sent to the speech recognition service, in seconds.
    pub max_command_secs: f32,
    /// How long one confirmation by the speech recognition service may take before it is
    /// given up.
    pub confirmation_deadline: Duration,
}

/// Shared by every connection: the loaded keyword spotter and the keyword list.
pub struct Engine {
    spotter: KeywordSpotter,
    keywords: Vec<Keyword>,
    vad: VadModelConfig,
    save_speech_dir: Option<PathBuf>,
    hear: Option<Arc<dyn Hear>>,
    deadline: Duration,
    max_command_samples: usize,
    min_silence_samples: usize,
}

impl Engine {
    pub fn load(cfg: &EngineCfg) -> Result<Self> {
        let keywords = keywords::parse(cfg.keywords.as_deref().unwrap_or(keywords::KEYWORDS));
        let spotter =
            KeywordSpotter::create(&spotter_config(cfg, &keywords)?).ok_or_else(|| {
                anyhow!(
                    "cannot load the keyword spotting model from {}",
                    cfg.model_dir.display()
                )
            })?;
        Ok(Self {
            spotter,
            keywords,
            vad: vad_config(cfg),
            save_speech_dir: cfg.save_speech_dir.clone(),
            hear: cfg
                .transcription_url
                .as_deref()
                .map(|url| Arc::new(UtteranceTranscriber::new(url)) as Arc<dyn Hear>),
            deadline: cfg.confirmation_deadline,
            max_command_samples: samples_in(cfg.max_command_secs),
            min_silence_samples: samples_in(cfg.min_silence_secs),
        })
    }

    /// Confirms utterances with `hear` in place of the speech recognition service.
    pub fn hearing(mut self, hear: Arc<dyn Hear>) -> Self {
        self.hear = Some(hear);
        self
    }

    pub fn session(&self) -> Result<Session<'_>> {
        self.session_for(0)
    }

    /// The session of connection `conn`, which names its threads and log lines.
    pub fn session_for(&self, conn: u64) -> Result<Session<'_>> {
        let vad = VoiceActivityDetector::create(&self.vad, VAD_BUFFER_SECS)
            .ok_or_else(|| anyhow!("cannot load the voice activity detector"))?;
        let worker = self
            .hear
            .clone()
            .map(|hear| ConfirmationWorker::start(conn, hear, self.deadline))
            .transpose()?;
        Ok(Session {
            engine: self,
            conn,
            worker,
            outbox: Outbox::default(),
            next_job: 0,
            jobs: HashMap::new(),
            utterances: HashMap::new(),
            current: None,
            next_utterance: 0,
            vad,
            pause: PauseWatch::new(self.min_silence_samples),
            stream: None,
            samples_seen: 0,
            vad_pending: Vec::new(),
            orderer: EventOrderer::default(),
            recent: RecentAudio::new(samples_in(RECENT_AUDIO_SECS)),
            matcher: PhraseMatcher::new(&[]),
            vad_fed: 0,
            vad_offset: 0,
            last_segment_end: 0,
        })
    }
}

/// One connection's audio: its clock, its keyword stream and its speech segments.
pub struct Session<'a> {
    engine: &'a Engine,
    conn: u64,
    /// Runs the confirmations by the speech recognition service; none without one.
    worker: Option<ConfirmationWorker>,
    outbox: Outbox,
    next_job: u64,
    /// What each confirmation asked for, by job number, until its answer is applied.
    jobs: HashMap<u64, JobContext>,
    /// The utterances with confirmations still to apply, and the one heard now.
    utterances: HashMap<u64, Running>,
    current: Option<u64>,
    next_utterance: u64,
    vad: VoiceActivityDetector,
    /// Ends an utterance at a quiet pause the detector still holds as speech.
    pause: PauseWatch,
    stream: Option<OnlineStream>,
    samples_seen: usize,
    vad_pending: Vec<f32>,
    orderer: EventOrderer,
    recent: RecentAudio,
    matcher: PhraseMatcher,
    /// Samples fed to the voice activity detector so far.
    vad_fed: usize,
    /// Samples fed before its last reset: its segment positions count from there.
    vad_offset: usize,
    /// Where the last utterance ended: the pre-roll of the next never reaches back past it.
    last_segment_end: usize,
}

/// An utterance still being spoken, searched for commands while it runs.
struct Running {
    /// Where it started, by sample position.
    start: usize,
    /// Its start was searched for "start".
    head_checked: bool,
    /// When to search its last seconds again.
    next_window: usize,
    /// Commands found up to this sample position are not reported again.
    reported_until: usize,
    /// The last window heard while it ran, and where that window started.
    last_window: Option<(Heard, usize)>,
    /// Confirmations of it whose answers are not applied yet.
    jobs: usize,
    /// It ended: its state goes once its last confirmation is applied.
    closed: bool,
}

impl Session<'_> {
    /// Spots `phrases` from now on.
    pub fn listen_for(&mut self, phrases: &[String]) -> Result<(), UnknownPhrases> {
        let selected = keywords::select(&self.engine.keywords, phrases)?;
        self.stream = Some(self.engine.spotter.create_stream_with_keywords(&selected));
        self.matcher = PhraseMatcher::new(phrases);
        Ok(())
    }

    /// Spots and detects speech frame by frame, so a phrase is placed within one frame of
    /// where it was decoded however much audio a message carries.
    pub fn accept(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        for frame in samples.chunks(DECODE_FRAME_SAMPLES) {
            self.recent.push(frame);
            let spotted = self.spot(frame);
            // A phrase goes out from the orderer at once only when it belongs to the segment
            // that ended last: the spotter may report it after a quiet pause closed that one.
            events.extend(self.outbox.send_in_last_segment(spotted));
            events.extend(self.detect_speech(frame));
            self.samples_seen += frame.len();
            self.watch_running();
            self.confirm_unheld();
            events.extend(self.take_answers());
        }
        events
    }

    fn spot(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        let Some(stream) = self.stream.as_ref() else {
            return Vec::new();
        };
        let spotter = &self.engine.spotter;
        stream.accept_waveform(SAMPLE_RATE as i32, samples);
        let position = seconds(self.samples_seen + samples.len());
        let mut events = Vec::new();
        while spotter.is_ready(stream) {
            spotter.decode(stream);
            let Some(result) = spotter.get_result(stream).filter(|r| !r.keyword.is_empty()) else {
                continue;
            };
            spotter.reset(stream);
            let phrase = keywords::phrase_of(&result.keyword);
            let (start, end) = phrase_span(position, &result.timestamps);
            log::info!("spotted \"{phrase}\" at {start:.2}–{end:.2}s");
            events.extend(self.orderer.phrase(Spotted { phrase, start, end }));
        }
        events
    }

    fn detect_speech(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        self.vad_pending.extend_from_slice(samples);
        let windows = self.vad_pending.len() / SILERO_WINDOW;
        for window in self.vad_pending.chunks_exact(SILERO_WINDOW).take(windows) {
            self.vad.accept_waveform(window);
        }
        self.vad_fed += windows * SILERO_WINDOW;
        self.vad_pending.drain(..windows * SILERO_WINDOW);
        let mut events = self.take_segments(None);
        if let Some(quiet_from) = self.pause.hear(samples, self.vad.detected()) {
            // WHY: after a quiet word the detector holds speech up to 0.55 s into the pause,
            // so a short pause before more speech would never end the utterance.
            self.vad.flush();
            events.extend(self.take_segments(Some(quiet_from)));
            self.pause.forget();
        }
        events
    }

    /// The events of the segments the detector closed; one closed by a quiet pause ends where
    /// the pause began, at `quiet_from`.
    fn take_segments(&mut self, quiet_from: Option<usize>) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        while let Some(segment) = self.vad.front() {
            self.vad.pop();
            let start = self.vad_offset + segment.start().max(0) as usize;
            let detected_end = start + segment.n().max(0) as usize;
            let end = quiet_from.map_or(detected_end, |q| q.clamp(start, detected_end));
            let bounds = Segment {
                start: seconds(start),
                end: seconds(end),
            };
            log::info!("speech {:.2}–{:.2}s", bounds.start, bounds.end);
            if let Some(dir) = self.engine.save_speech_dir.as_deref() {
                save_speech(dir, bounds.start, segment.samples());
            }
            let segment_events = self.orderer.segment(bounds);
            let quiet_level = self.pause.quiet_level();
            events.extend(self.close_segment(
                start,
                end,
                bounds,
                segment_events,
                quiet_level,
                quiet_from.is_some(),
            ));
            self.last_segment_end = end;
            // WHY: after some utterances the detector's state stops it noticing quiet
            // speech; a reset in the silence that closed the segment loses nothing.
            self.vad.reset();
            self.vad_offset = self.vad_fed;
            self.pause.forget();
        }
        events
    }

    /// Searches the utterance being spoken: its start for "start", and every second its
    /// last seconds for a doubled or chained command; a check is skipped while another one
    /// waits for its answer.
    fn watch_running(&mut self) {
        if self.worker.is_none() || !self.vad.detected() {
            self.close_current();
            return;
        }
        let now = self.samples_seen;
        let floor = self.last_segment_end;
        let id = self.current_or_new(now.saturating_sub(samples_in(ONSET_SECS)).max(floor));
        let Some(&Running {
            start,
            head_checked,
            next_window,
            ..
        }) = self.utterances.get(&id)
        else {
            return;
        };
        if !head_checked && now - start >= samples_in(HEAD_CHECK_SECS) && !self.outbox.check_busy()
        {
            self.mark(id, |r| r.head_checked = true);
            let from = self.pre_rolled(start);
            self.ask(
                JobKind::Head,
                from,
                now,
                JobContext::Head {
                    utterance: id,
                    from,
                },
            );
        }
        if now >= next_window {
            self.mark(id, |r| r.next_window = now + samples_in(WINDOW_EVERY_SECS));
            if self.outbox.check_busy() {
                log::debug!(
                    "conn={} utterance={id} window skipped: a check still waits",
                    self.conn
                );
                return;
            }
            let from = now
                .saturating_sub(samples_in(WINDOW_SECS))
                .max(self.pre_rolled(start));
            self.ask(
                JobKind::Window,
                from,
                now,
                JobContext::Window {
                    utterance: id,
                    from,
                },
            );
        }
    }

    /// The utterance heard now, started at `start` when there is none yet.
    fn current_or_new(&mut self, start: usize) -> u64 {
        if let Some(id) = self.current {
            return id;
        }
        let id = self.new_utterance(start, samples_in(FIRST_WINDOW_SECS), false);
        self.current = Some(id);
        id
    }

    fn new_utterance(
        &mut self,
        start: usize,
        first_window_after: usize,
        head_checked: bool,
    ) -> u64 {
        let id = self.next_utterance;
        self.next_utterance += 1;
        let next_window = self.samples_seen.saturating_add(first_window_after);
        self.utterances.insert(
            id,
            Running {
                start,
                head_checked,
                next_window,
                reported_until: 0,
                last_window: None,
                jobs: 0,
                closed: false,
            },
        );
        id
    }

    fn mark(&mut self, utterance: u64, change: impl FnOnce(&mut Running)) {
        if let Some(running) = self.utterances.get_mut(&utterance) {
            change(running);
        }
    }

    fn close_current(&mut self) {
        if let Some(id) = self.current.take() {
            self.mark(id, |r| r.closed = true);
            self.forget_if_done(id);
        }
    }

    fn forget_if_done(&mut self, utterance: u64) {
        if self
            .utterances
            .get(&utterance)
            .is_some_and(|r| r.closed && r.jobs == 0)
        {
            self.utterances.remove(&utterance);
        }
    }

    /// `start` moved back by the pre-roll, never past the end of the utterance before.
    fn pre_rolled(&self, start: usize) -> usize {
        start
            .saturating_sub(samples_in(PRE_ROLL_SECS))
            .max(self.last_segment_end.min(start))
    }
}

fn samples_in(secs: f32) -> usize {
    (secs * SAMPLE_RATE as f32) as usize
}

/// Writes one speech segment as 16 kHz mono 16-bit WAV; a failure is only logged.
fn save_speech(dir: &Path, start: f32, samples: &[f32]) {
    let path = dir.join(format!("speech-{}-{start:08.2}s.wav", std::process::id()));
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let written = hound::WavWriter::create(&path, spec).and_then(|mut writer| {
        for sample in samples {
            writer.write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
        }
        writer.finalize()
    });
    match written {
        Ok(()) => log::info!("saved {}", path.display()),
        Err(e) => log::warn!("cannot save {}: {e}", path.display()),
    }
}

/// Where a phrase spotted at `position` (seconds of audio fed so far) sits in the stream.
/// The spotter's token times only tell how long the phrase lasted: they do not count from
/// the start of the stream, so the phrase is placed to end where the spotter has decoded to.
pub fn phrase_span(position: f32, token_times: &[f32]) -> (f32, f32) {
    let token_times = without_stray_lead(token_times);
    let first = token_times.first().copied().unwrap_or_default();
    let last = token_times.last().copied().unwrap_or_default();
    let duration = (last - first) + FIRST_TOKEN_SECS + LAST_TOKEN_SECS;
    let end = position - DETECTION_LAG_SECS;
    ((end - duration).max(0.0), end)
}

/// The token times from the first one that is followed closely by the next: a first token
/// heard in noise fires well before the rest of the phrase and would stretch it.
fn without_stray_lead(token_times: &[f32]) -> &[f32] {
    let first_close = token_times
        .windows(2)
        .position(|pair| pair[1] - pair[0] <= MAX_TOKEN_GAP_SECS)
        .unwrap_or(0);
    &token_times[first_close..]
}

fn seconds(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}

fn spotter_config(cfg: &EngineCfg, keywords: &[Keyword]) -> Result<KeywordSpotterConfig> {
    let mut config = KeywordSpotterConfig::default();
    let transducer = &mut config.model_config.transducer;
    transducer.encoder = Some(model_file(&cfg.model_dir, "encoder")?);
    transducer.decoder = Some(model_file(&cfg.model_dir, "decoder")?);
    transducer.joiner = Some(model_file(&cfg.model_dir, "joiner")?);
    config.model_config.tokens = Some(path_string(&cfg.model_dir.join("tokens.txt")));
    config.model_config.num_threads = cfg.threads;
    config.keywords_buf = Some(keywords::all_lines(keywords));
    Ok(config)
}

fn vad_config(cfg: &EngineCfg) -> VadModelConfig {
    VadModelConfig {
        silero_vad: SileroVadModelConfig {
            model: Some(path_string(&cfg.vad_model)),
            threshold: VAD_SPEECH_THRESHOLD,
            min_silence_duration: cfg.min_silence_secs,
            min_speech_duration: MIN_SPEECH_SECS,
            window_size: SILERO_WINDOW as i32,
            max_speech_duration: MAX_SPEECH_SECS,
        },
        sample_rate: SAMPLE_RATE as i32,
        num_threads: 1,
        ..VadModelConfig::default()
    }
}

/// The int8 model file whose name starts with `part`.
fn model_file(dir: &Path, part: &str) -> Result<String> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("read model directory {}", dir.display()))?;
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.starts_with(part) && name.ends_with(".int8.onnx")
        })
        .map(|p| path_string(&p))
        .ok_or_else(|| anyhow!("no {part}*.int8.onnx in {}", dir.display()))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    fn assert_close(actual: (f32, f32), expected: (f32, f32)) {
        assert!(
            (actual.0 - expected.0).abs() < TOLERANCE && (actual.1 - expected.1).abs() < TOLERANCE,
            "got {actual:?}, expected {expected:?}"
        );
    }

    #[test]
    fn should_end_the_phrase_where_the_spotter_has_decoded_to() {
        // Given token times of a phrase that took 0.6 s between its first and last token
        let token_times = [0.32, 0.5, 0.92];
        // When
        let span = phrase_span(27.4, &token_times);
        // Then
        let end = 27.4 - DETECTION_LAG_SECS;
        let duration = 0.6 + FIRST_TOKEN_SECS + LAST_TOKEN_SECS;
        assert_close(span, (end - duration, end));
    }

    #[test]
    fn should_place_a_phrase_late_in_a_long_stream_whatever_its_token_times_count_from() {
        // Given the same phrase reported with token times from two different origins
        let early = phrase_span(60.0, &[0.3, 0.9]);
        let late = phrase_span(60.0, &[45.3, 45.9]);
        // Then both sit at the end of the audio fed so far
        assert_close(early, late);
        assert!(early.0 > 58.0, "{early:?}");
    }

    #[test]
    fn should_not_count_a_stray_first_token_heard_in_noise() {
        // Given the first token fired in noise half a second before the rest of the phrase
        let with_stray = phrase_span(27.0, &[0.32, 0.8, 0.88, 1.08]);
        let without = phrase_span(27.0, &[0.8, 0.88, 1.08]);
        // Then
        assert_close(with_stray, without);
    }

    #[test]
    fn should_not_place_a_phrase_before_the_stream_started() {
        // When
        let (start, _) = phrase_span(0.5, &[0.1, 0.8]);
        // Then
        assert_eq!(start, 0.0);
    }

    #[test]
    fn should_give_a_phrase_without_token_times_the_token_tails_only() {
        // When
        let (start, end) = phrase_span(10.0, &[]);
        // Then
        assert!((end - start - (FIRST_TOKEN_SECS + LAST_TOKEN_SECS)).abs() < TOLERANCE);
    }
}
