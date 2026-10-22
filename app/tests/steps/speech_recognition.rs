//! Drives the pipeline through one live dictation: a stand-in recorder delivers chunks, a
//! stand-in speech recognition service answers each chunk with a fixed transcript, and the
//! pasted text is captured. Refinement passes the raw transcript through unchanged, so the
//! pasted text is the merged raw transcript.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, PipelineInput, SoundPlayer, TextPaster, Transcriber,
    VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, PttSignal, RawTranscript, RecordingMode,
};
use voicepuncher_e2e_test::speech::repeated_phrases;

use crate::IntegrationWorld;

pub mod losses;

const CHUNK_THRESHOLD_SECS: u32 = 30;

/// One dictation: what the service answers per chunk, and the text the user spoke.
#[derive(Debug, Clone, Default)]
pub struct Dictation {
    chunk_texts: Vec<&'static str>,
    spoken: &'static str,
    /// The chunk the service fails on every attempt, counting from 0.
    failing_chunk: Option<usize>,
}

fn same_overlap() -> Dictation {
    Dictation {
        chunk_texts: vec![
            "This morning I went through the task queue and realised",
            "and realised that half of it was out of date.",
            "out of date. We agreed with the team to review the list.",
        ],
        spoken: "This morning I went through the task queue and realised that half of it was out of date. We agreed with the team to review the list.",
        failing_chunk: None,
    }
}

#[expect(clippy::too_many_lines, reason = "size baseline")]
fn differing_overlap(language: &str, difference: &str) -> Result<Dictation> {
    let (chunk_texts, spoken) = match (language, difference) {
        ("Russian", "other punctuation and case") => (
            vec![
                "Я добавил индекс, перезапустил сервис и время ответа упало.",
                "Время ответа упало с двух секунд до сорока миллисекунд.",
            ],
            "Я добавил индекс, перезапустил сервис и время ответа упало с двух секунд до сорока миллисекунд.",
        ),
        ("Russian", "another spelling") => (
            vec![
                "иметь понятный интерфейс. Ещё нужно написать тесты",
                "Еще нужно написать тесты на возврат денег, потому что",
            ],
            "иметь понятный интерфейс. Ещё нужно написать тесты на возврат денег, потому что",
        ),
        ("Russian", "a word cut short at the chunk edge") => (
            vec![
                "Вечером мы обсуждали, как разбить большой модуль оплат",
                "как разбить большой модуль оплаты на несколько маленьких частей.",
            ],
            "Вечером мы обсуждали, как разбить большой модуль оплаты на несколько маленьких частей.",
        ),
        ("Russian", "a different reading of a word") => (
            vec![
                "Я добавил индекс, перезапустил сервис и время ответа упало с двух секунд.",
                "время ответа упало с 2 секунд до 40 миллисекунд.",
            ],
            "Я добавил индекс, перезапустил сервис и время ответа упало с 2 секунд до 40 миллисекунд.",
        ),
        ("English", "other punctuation and case") => (
            vec![
                "In the evening we discussed how to split the large payment module.",
                "Payment module into several small parts. Each part",
            ],
            "In the evening we discussed how to split the large payment module into several small parts. Each part",
        ),
        ("English", "another spelling") => (
            vec![
                "We agreed with the team to review the list every Friday after-noon.",
                "every Friday afternoon. Then I opened the server logs",
            ],
            "We agreed with the team to review the list every Friday afternoon. Then I opened the server logs",
        ),
        ("English", "a word cut short at the chunk edge") => (
            vec![
                "I went through the task queue and realised that half of it was out of da",
                "half of it was out of date. We agreed with the team",
            ],
            "I went through the task queue and realised that half of it was out of date. We agreed with the team",
        ),
        ("English", "a different reading of a word") => (
            vec![
                "and the response time dropped from two seconds to forty.",
                "dropped from 2 seconds to 40 milliseconds. In the evening",
            ],
            "and the response time dropped from 2 seconds to 40 milliseconds. In the evening",
        ),
        _ => bail!("no {language} dictation with {difference}"),
    };
    Ok(Dictation { chunk_texts, spoken, failing_chunk: None })
}

fn no_shared_words() -> Dictation {
    Dictation {
        chunk_texts: vec![
            "It was a long day at the office.",
            "Tomorrow we start early and review the plan.",
        ],
        spoken: "It was a long day at the office. Tomorrow we start early and review the plan.",
        failing_chunk: None,
    }
}

fn three_chunks() -> Dictation {
    Dictation {
        chunk_texts: vec![
            "It was a long day at the office.",
            "We talked about the release for an hour.",
            "Tomorrow we start early and review the plan.",
        ],
        spoken: "It was a long day at the office. Tomorrow we start early and review the plan.",
        failing_chunk: None,
    }
}

#[given("a dictation long enough for three chunks")]
fn three_chunk_dictation(world: &mut IntegrationWorld) {
    world.dictation = Some(three_chunks());
}

#[given("the speech recognition service fails every attempt for the second chunk")]
fn second_chunk_fails(world: &mut IntegrationWorld) -> Result<()> {
    crate::steps::voice_control::pipeline_log();
    world
        .dictation
        .as_mut()
        .context("no dictation")?
        .failing_chunk = Some(1);
    Ok(())
}

#[then("the text of the first and third chunks is pasted")]
fn first_and_third_pasted(world: &mut IntegrationWorld) -> Result<()> {
    pasted_as_spoken(world)
}

#[then("the lost second chunk is logged")]
fn lost_chunk_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let errors = crate::steps::voice_control::pipeline_log()
        .errors
        .lock()
        .unwrap()
        .clone();
    if !errors.iter().any(|e| e.starts_with("chunk 2 lost")) {
        bail!("no error names the lost second chunk: {errors:?}");
    }
    Ok(())
}

#[given("the user dictates for longer than the chunk threshold")]
fn long_dictation(_world: &mut IntegrationWorld) {}

#[given("the service transcribes the overlap of every two chunks the same way")]
fn same_overlap_transcripts(world: &mut IntegrationWorld) {
    world.dictation = Some(same_overlap());
}

#[given(regex = r"^a (Russian|English) dictation longer than the chunk threshold$")]
fn long_dictation_in(world: &mut IntegrationWorld, language: String) {
    world.dictation_language = Some(language);
}

#[given(regex = r"^the service transcribes the overlap differently in the two chunks: (.+)$")]
fn differing_overlap_transcripts(world: &mut IntegrationWorld, difference: String) -> Result<()> {
    let language = world
        .dictation_language
        .clone()
        .context("no dictation language")?;
    world.dictation = Some(differing_overlap(&language, &difference)?);
    Ok(())
}

#[given(
    "a dictation longer than the chunk threshold whose chunk transcripts share no words at the seam"
)]
fn disjoint_transcripts(world: &mut IntegrationWorld) {
    world.dictation = Some(no_shared_words());
}

#[when("the user stops the dictation")]
fn stop_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let dictation = world.dictation.clone().context("no dictation")?;
    let pasted = Arc::new(Mutex::new(Vec::new()));
    let mut pipeline = Pipeline::new(
        Box::new(ChunkingRecorder { chunks: dictation.chunk_texts.len() }),
        Box::new(FixedTranscripts(
            dictation.chunk_texts.clone(),
            dictation.failing_chunk,
        )),
        Box::new(PassthroughTextRefiner),
        Box::new(CapturingPaster(pasted.clone())),
        Box::new(Silent),
        Box::new(Silent),
        PipelineConfig {
            min_press_ms: 0,
            recording_mode: RecordingMode::Toggle,
            chunking: ChunkingCfg {
                chunk_threshold_secs: CHUNK_THRESHOLD_SECS,
                ..ChunkingCfg::default()
            },
        },
    );
    let (tx, rx) = mpsc::channel();
    tx.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    tx.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    drop(tx);
    // `run` only returns once the hotkey channel closes, after both presses were handled
    let _ = pipeline.run(rx);
    let pasted = pasted.lock().unwrap();
    world.pasted = Some(pasted.first().cloned().context("nothing was pasted")?);
    Ok(())
}

#[then("the pasted text contains every spoken word once, in order")]
#[then("the pasted text contains every word of every chunk, in order")]
#[then("the words before and after the overlap are kept")]
fn pasted_as_spoken(world: &mut IntegrationWorld) -> Result<()> {
    let (spoken, pasted) = spoken_and_pasted(world)?;
    if pasted != spoken {
        bail!("pasted text differs from the speech\n  spoken: {spoken}\n  pasted: {pasted}");
    }
    Ok(())
}

#[then("the overlapping words appear once in the pasted text")]
fn overlap_once(world: &mut IntegrationWorld) -> Result<()> {
    let (spoken, pasted) = spoken_and_pasted(world)?;
    let repeated = repeated_phrases(spoken, pasted);
    if !repeated.is_empty() {
        bail!("phrases repeated at the seam: {repeated:?}\n  pasted: {pasted}");
    }
    Ok(())
}

fn spoken_and_pasted(world: &IntegrationWorld) -> Result<(&'static str, &str)> {
    let spoken = world.dictation.as_ref().context("no dictation")?.spoken;
    let pasted = world.pasted.as_deref().context("nothing was pasted")?;
    Ok((spoken, pasted))
}

/// Delivers `chunks` one-second live chunks; the first sample numbers the chunk.
struct ChunkingRecorder {
    chunks: usize,
}

impl AudioRecorder for ChunkingRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(Vec::new()))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        let (tx, rx) = mpsc::channel();
        for n in 0..self.chunks {
            let mut samples = vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize];
            samples[0] = n as i16;
            tx.send(LiveChunk {
                audio: AudioBuffer16k::new(samples),
                enqueued_at: Instant::now(),
            })?;
        }
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {}
}

/// Answers chunk n with the n-th transcript, and fails the chunk it is told to fail.
struct FixedTranscripts(Vec<&'static str>, Option<usize>);

impl Transcriber for FixedTranscripts {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let n = audio.samples()[0] as usize;
        if self.1 == Some(n) {
            bail!("speech recognition request failed: every attempt timed out");
        }
        let text = self.0.get(n).context("no transcript for this chunk")?;
        Ok(Some(RawTranscript::new((*text).to_owned())?))
    }
}

struct CapturingPaster(Arc<Mutex<Vec<String>>>);

impl TextPaster for CapturingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.0.lock().unwrap().push(text.as_str().to_owned());
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        bail!("the text was left in the clipboard instead of pasted")
    }
}

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}

impl VisualIndicator for Silent {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
}
