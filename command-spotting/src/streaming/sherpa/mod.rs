//! The sherpa-onnx models behind the streaming ports: a streaming transducer recognizer and
//! the Silero voice activity detector as the gate.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{
    OnlineRecognizer, OnlineRecognizerConfig, OnlineStream, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

use crate::audio::SAMPLE_RATE;

use super::ports::{Decoded, RecognizerStream, SpeechGate, StreamingModel, GATE_WINDOW};

/// How much louder the gate hears the stream than the microphone gave it: as the keyword
/// spotting engine's detector, so the user's quietest commands still open an utterance.
const GATE_GAIN: f32 = 1.5;
const GATE_THRESHOLD: f32 = 0.5;
/// Shorter speech is noise to the gate.
const GATE_MIN_SPEECH_SECS: f32 = 0.15;
/// The gate's own hold after speech; the session's pause counts from its end.
const GATE_MIN_SILENCE_SECS: f32 = 0.1;
const GATE_MAX_SPEECH_SECS: f32 = 20.0;
const GATE_BUFFER_SECS: f32 = 30.0;

/// A streaming transducer recognizer loaded from its model directory.
pub struct SherpaModel {
    name: String,
    respelled: bool,
    recognizer: OnlineRecognizer,
}

impl SherpaModel {
    /// Loads the model in `dir` (`encoder`, `decoder`, `joiner` and `tokens.txt`), named
    /// `name`; with `respelled`, its Cyrillic command words are respelled.
    pub fn load(name: &str, dir: &Path, respelled: bool, threads: i32) -> Result<Self> {
        let mut config = OnlineRecognizerConfig::default();
        let transducer = &mut config.model_config.transducer;
        transducer.encoder = Some(model_file(dir, "encoder")?);
        transducer.decoder = Some(model_file(dir, "decoder")?);
        transducer.joiner = Some(model_file(dir, "joiner")?);
        config.model_config.tokens = Some(path_string(&dir.join("tokens.txt")));
        config.model_config.num_threads = threads;
        config.decoding_method = Some("greedy_search".to_owned());
        let recognizer = OnlineRecognizer::create(&config)
            .ok_or_else(|| anyhow!("cannot load the streaming model from {}", dir.display()))?;
        Ok(Self { name: name.to_owned(), respelled, recognizer })
    }
}

impl StreamingModel for SherpaModel {
    fn name(&self) -> &str {
        &self.name
    }

    fn respelled(&self) -> bool {
        self.respelled
    }

    fn stream(&self) -> Box<dyn RecognizerStream + '_> {
        Box::new(SherpaStream {
            recognizer: &self.recognizer,
            stream: self.recognizer.create_stream(),
        })
    }
}

struct SherpaStream<'a> {
    recognizer: &'a OnlineRecognizer,
    stream: OnlineStream,
}

impl RecognizerStream for SherpaStream<'_> {
    fn accept(&mut self, samples: &[f32]) {
        self.stream.accept_waveform(rate(), samples);
    }

    fn finish(&mut self) {
        self.stream.input_finished();
    }

    fn decode(&mut self) {
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
        }
    }

    fn decoded(&self) -> Decoded {
        self.recognizer
            .get_result(&self.stream)
            .map(|result| Decoded {
                times: result.timestamps.unwrap_or_default(),
                tokens: result.tokens,
            })
            .unwrap_or_default()
    }
}

/// The Silero voice activity detector of one connection.
pub struct SileroGate {
    vad: VoiceActivityDetector,
}

impl SileroGate {
    pub fn load(model: &Path) -> Result<Self> {
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(path_string(model)),
                threshold: GATE_THRESHOLD,
                min_silence_duration: GATE_MIN_SILENCE_SECS,
                min_speech_duration: GATE_MIN_SPEECH_SECS,
                window_size: i32::try_from(GATE_WINDOW).unwrap_or(i32::MAX),
                max_speech_duration: GATE_MAX_SPEECH_SECS,
            },
            sample_rate: rate(),
            num_threads: 1,
            ..VadModelConfig::default()
        };
        let vad = VoiceActivityDetector::create(&config, GATE_BUFFER_SECS).ok_or_else(|| {
            anyhow!(
                "cannot load the voice activity detector {}",
                model.display()
            )
        })?;
        Ok(Self { vad })
    }
}

impl SpeechGate for SileroGate {
    fn hears(&mut self, window: &[f32]) -> bool {
        let raised: Vec<f32> = window
            .iter()
            .map(|s| (s * GATE_GAIN).clamp(-1.0, 1.0))
            .collect();
        self.vad.accept_waveform(&raised);
        // WHY: the gate needs only whether speech goes on; the segments it keeps would grow.
        self.vad.clear();
        self.vad.detected()
    }
}

/// The file in `dir` whose name starts with `part`, its int8 form first.
fn model_file(dir: &Path, part: &str) -> Result<String> {
    let names: Vec<String> = onnx_files(dir)?
        .into_iter()
        .filter(|name| name.starts_with(part))
        .collect();
    names
        .iter()
        .find(|name| name.ends_with(".int8.onnx"))
        .or_else(|| names.first())
        .map(|name| path_string(&dir.join(name)))
        .ok_or_else(|| anyhow!("no {part}*.onnx in {}", dir.display()))
}

/// The names of the `.onnx` files in `dir`.
fn onnx_files(dir: &Path) -> Result<Vec<String>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("read model directory {}", dir.display()))?;
    Ok(entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("onnx"))
        })
        .collect())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn rate() -> i32 {
    i32::try_from(SAMPLE_RATE).unwrap_or(i32::MAX)
}
