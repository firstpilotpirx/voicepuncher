//! Runs the keyword spotter and the voice activity detector over one connection's audio.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, OnlineStream, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

use crate::confirm::{RecentAudio, UtteranceTranscriber};
use crate::keywords::{self, Keyword, UnknownPhrases};
use crate::matcher::PhraseMatcher;
use crate::orderer::{EventOrderer, Segment, Spotted};
use crate::protocol::ServerEvent;

pub const SAMPLE_RATE: u32 = 16_000;
const SILERO_WINDOW: usize = 512;
const VAD_BUFFER_SECS: f32 = 30.0;
const VAD_SPEECH_THRESHOLD: f32 = 0.5;
const MIN_SPEECH_SECS: f32 = 0.25;
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
}

/// Shared by every connection: the loaded keyword spotter and the keyword list.
pub struct Engine {
    spotter: KeywordSpotter,
    keywords: Vec<Keyword>,
    vad: VadModelConfig,
    save_speech_dir: Option<PathBuf>,
    transcriber: Option<UtteranceTranscriber>,
    max_command_samples: usize,
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
            transcriber: cfg
                .transcription_url
                .as_deref()
                .map(UtteranceTranscriber::new),
            max_command_samples: samples_in(cfg.max_command_secs),
        })
    }

    pub fn session(&self) -> Result<Session<'_>> {
        let vad = VoiceActivityDetector::create(&self.vad, VAD_BUFFER_SECS)
            .ok_or_else(|| anyhow!("cannot load the voice activity detector"))?;
        Ok(Session {
            engine: self,
            vad,
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
    vad: VoiceActivityDetector,
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
            events.extend(self.spot(frame));
            events.extend(self.detect_speech(frame));
            self.samples_seen += frame.len();
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
        let mut events = Vec::new();
        while let Some(segment) = self.vad.front() {
            self.vad.pop();
            let start = self.vad_offset + segment.start().max(0) as usize;
            let bounds = Segment {
                start: seconds(start),
                end: seconds(start + segment.n().max(0) as usize),
            };
            log::info!("speech {:.2}–{:.2}s", bounds.start, bounds.end);
            if let Some(dir) = self.engine.save_speech_dir.as_deref() {
                save_speech(dir, bounds.start, segment.samples());
            }
            events.extend(self.orderer.segment(bounds));
            let end = start + segment.n().max(0) as usize;
            events.extend(self.confirm(start, end, bounds));
            self.last_segment_end = end;
            // WHY: after some utterances the detector's state stops it noticing quiet
            // speech; a reset in the silence that closed the segment loses nothing.
            self.vad.reset();
            self.vad_offset = self.vad_fed;
        }
        events
    }

    /// The command phrase the whole utterance from `start` to `end` says, according to the
    /// speech recognition service; utterances too long for a command are not sent.
    fn confirm(&self, start: usize, end: usize, bounds: Segment) -> Option<ServerEvent> {
        let transcriber = self.engine.transcriber.as_ref()?;
        if end - start > self.engine.max_command_samples {
            return None;
        }
        let pre_roll_start = start
            .saturating_sub(samples_in(PRE_ROLL_SECS))
            .max(self.last_segment_end.min(start));
        let audio = self
            .recent
            .between(pre_roll_start, end + samples_in(POST_ROLL_SECS));
        let text = transcriber
            .transcribe(&audio, SAMPLE_RATE)
            .map_err(|e| log::warn!("cannot confirm the utterance: {e:#}"))
            .ok()?;
        let phrase = self.matcher.phrase_in(&text);
        log::info!("heard {text:?} → {phrase:?}");
        Some(ServerEvent::Phrase {
            phrase: phrase?,
            start: bounds.start,
            end: bounds.end,
        })
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
