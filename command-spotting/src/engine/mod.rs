//! Runs the keyword spotter and the voice activity detector over one connection's audio.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

use crate::confirm::{Hear, Heard, RecentAudio, UtteranceTranscriber};
use crate::history::{CommandHistory, HistoryCfg, Recorder};
use crate::keywords::{self, Keyword, UnknownPhrases};
use crate::matcher::PhraseMatcher;
use crate::orderer::{EventOrderer, Segment};
use crate::protocol::{PhraseSelection, ServerEvent};
use crate::worker::{ConfirmationWorker, JobKind};

mod clip;
mod confirming;
mod heads;
use heads::HeadCheck;
mod pause;
mod queued;
mod recording;
mod speech;
pub mod spotted;
mod spotting;
mod stall;
mod state;

use confirming::{ClosedSegment, JobContext};
use pause::PauseWatch;
use spotting::Spotting;
use state::{Detector, Hearings, Jobs, Utterances};

mod model_config;
pub use crate::audio::SAMPLE_RATE;
use crate::conversions::{saturating_i16, saturating_usize};
use model_config::{spotter_config, vad_config};

const SILERO_WINDOW: usize = 512;
const VAD_BUFFER_SECS: f32 = 30.0;
const VAD_SPEECH_THRESHOLD: f32 = 0.5;
/// How much louder the voice activity detector hears the stream than the microphone gave it:
/// 3.5 dB, so a command said as quietly as the user's quietest ones (loudest 10 ms at RMS
/// 0.0045) still reaches the detector's threshold. From about 1.8 times, a click in a 0.32 s
/// pause between two of the user's commands holds them in one segment.
const DETECTOR_GAIN: f32 = 1.5;
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
/// Longest gap between two tokens of one spoken phrase: a quiet "start" is decoded 0.32 s
/// before "dictation".
const MAX_TOKEN_GAP_SECS: f32 = 0.4;
/// Audio kept for the speech recognition of an utterance after its segment closed, and for
/// the checks waiting their turn behind slow confirmations: several deadlines of them.
const RECENT_AUDIO_SECS: f32 = 30.0;
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
/// How long after an utterance starts its start is first searched for a command opening
/// speech, when the app asks for an early start: the keyword spotter misses most of the user's
/// "start", so no spotted check comes.
const EARLY_HEAD_CHECK_SECS: f32 = 1.0;
/// How far before the end of its utterance a command heard while it ran may end and still
/// end the utterance: the detector holds speech this much past the last word.
const TAIL_SLACK_SECS: f32 = 0.6;
/// How late the voice activity detector notices speech has started.
const ONSET_SECS: f32 = 0.3;
/// How long a spotted trigger is kept for the segment it lies in.
const TRIGGERS_KEPT_SECS: f32 = 10.0;

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
    /// Where every utterance with a command heard is kept; none: nothing is kept.
    pub command_history: Option<HistoryCfg>,
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
    history: Option<CommandHistory>,
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
                .map(|url| UtteranceTranscriber::new(url).map(|t| Arc::new(t) as Arc<dyn Hear>))
                .transpose()?,
            deadline: cfg.confirmation_deadline,
            max_command_samples: samples_in(cfg.max_command_secs),
            min_silence_samples: samples_in(cfg.min_silence_secs),
            history: cfg
                .command_history
                .clone()
                .map(CommandHistory::start)
                .transpose()?,
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
        let start = |hear: Arc<dyn Hear>| ConfirmationWorker::start(conn, hear, self.deadline);
        let worker = self.hear.clone().map(start).transpose()?;
        Ok(Session {
            engine: self,
            conn,
            worker,
            jobs: Jobs::default(),
            hearings: Hearings::default(),
            utterances: Utterances::default(),
            detector: Detector::new(vad, PauseWatch::new(self.min_silence_samples)),
            spotting: Spotting::default(),
            orderer: EventOrderer::default(),
            recent: RecentAudio::new(samples_in(RECENT_AUDIO_SECS)),
            matcher: Arc::new(PhraseMatcher::new(&[])),
            recorder: self.history.as_ref().map(|_| Recorder::new(conn)),
        })
    }
}

/// One connection's audio: its clock, its keyword stream and its speech segments.
pub struct Session<'a> {
    engine: &'a Engine,
    conn: u64,
    /// Runs the confirmations by the speech recognition service; none without one.
    worker: Option<ConfirmationWorker>,
    jobs: Jobs,
    hearings: Hearings,
    utterances: Utterances,
    detector: Detector,
    spotting: Spotting,
    orderer: EventOrderer,
    recent: RecentAudio,
    /// Shared with every job asked under it, so a late answer knows the phrases of its state.
    matcher: Arc<PhraseMatcher>,
    /// Gathers the command history's entries; none while the engine keeps no history.
    recorder: Option<Recorder>,
}

/// An utterance still being spoken, searched for commands while it runs.
struct Running {
    /// Where it started, by sample position.
    start: usize,
    /// How far its start was searched for a command opening speech.
    head: HeadCheck,
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
    /// Its segment's confirmation was applied: a check of it answered later is stale.
    segment_answered: bool,
}

impl Session<'_> {
    /// Spots `phrases` from now on.
    pub fn listen_for(&mut self, phrases: &[String]) -> Result<(), UnknownPhrases> {
        self.select(&PhraseSelection { phrases: phrases.to_vec(), early_start: None })
    }

    /// Spots the phrases of `selection` from now on, searching the start of an utterance early
    /// as it says.
    pub fn select(&mut self, selection: &PhraseSelection) -> Result<(), UnknownPhrases> {
        let phrases = &selection.phrases;
        let selected = keywords::select(&self.engine.keywords, phrases)?;
        self.spotting.phrases = Some(self.engine.spotter.create_stream_with_keywords(&selected));
        self.spotting.triggers = keywords::select_triggers(&self.engine.keywords, phrases)
            .filter(|_| self.engine.hear.is_some())
            .map(|lines| self.engine.spotter.create_stream_with_keywords(&lines));
        self.matcher =
            Arc::new(PhraseMatcher::new(phrases).with_early_start(selection.early_start));
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
            events.extend(self.jobs.outbox.send_in_last_segment(spotted));
            events.extend(self.detect_speech(frame));
            self.detector.samples_seen += frame.len();
            self.watch_running();
            self.confirm_unheld();
            events.extend(self.take_answers());
            self.ask_spotted_check();
        }
        self.recorded(events, false)
    }
}

fn samples_in(secs: f32) -> usize {
    saturating_usize(secs * SAMPLE_RATE as f32)
}

/// `samples` as loud as the voice activity detector hears them.
fn raised(samples: &[f32]) -> Vec<f32> {
    samples
        .iter()
        .map(|s| (s * DETECTOR_GAIN).clamp(-1.0, 1.0))
        .collect()
}

/// Samples the detector heard, back at the level the microphone gave them.
fn as_given(samples: &[f32]) -> Vec<f32> {
    samples.iter().map(|s| s / DETECTOR_GAIN).collect()
}

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
            writer.write_sample(saturating_i16(
                sample.clamp(-1.0, 1.0) * f32::from(i16::MAX),
            ))?;
        }
        writer.finalize()
    });
    match written {
        Ok(()) => log::info!("saved {}", path.display()),
        Err(e) => log::warn!("cannot save {}: {e}", path.display()),
    }
}

/// Where a phrase spotted at `position` (seconds of audio fed so far) sits in the stream.
///
/// The spotter's token times only tell how long the phrase lasted: they do not count from the start
/// of the stream, so the phrase is placed to end where the spotter has decoded to.
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

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
