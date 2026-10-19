//! Drives the real pipeline on its own thread with the real dictation archive over a
//! temporary folder, and reads the archive entries back from that folder.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::adapters::secondary::archiving::filesystem::{
    ArchiveSetup, FileSystemDictationArchive,
};
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, PipelineInput, SoundPlayer, TextPaster,
    TextRefiner, Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, CommandPlacement, FailureNotice, PttSignal, RawTranscript,
    RecordingMode, SpottingEvent, VoiceCommand,
};

use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

const PHRASE: &str = "meeting moved to friday";
const SECOND_PHRASE: &str = "call me back tomorrow";
const CHUNK_TEXTS: [&str; 3] = ["one two three", "four five six", "seven eight nine"];
const STITCHED_CHUNKS: &str = "one two three four five six seven eight nine";
/// What speech recognition hears of "start", then "stop" said with nothing in between.
const ONLY_COMMAND_WORDS: &str = "Start stop.";
const DICTATION_MODEL: &str = "ggml-large-v3-turbo-q5_0.bin";
const REFINEMENT_MODEL: &str = "Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
const ARCHIVE_SIZE_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ENTRY_FILE: &str = "entry.json";
const AUDIO_FILE: &str = "audio.wav";
/// The start of the warning the pipeline logs when an entry cannot be written.
const NOT_ARCHIVED_WARNING: &str = "dictation not archived";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(5);
/// How long ago the voice commands of a scenario were said, so every moment is in the past.
const COMMANDS_SAID_AGO: Duration = Duration::from_secs(3);
const COMMAND_LENGTH: Duration = Duration::from_millis(500);
/// When "stop" is said after the start of "start".
const STOP_AFTER: Duration = Duration::from_secs(2);
/// Longer than the one-second parts every dictation is recorded in.
const MIN_PRESS: Duration = Duration::from_secs(5);
const SAMPLES_PER_PART: usize = AudioBuffer16k::SAMPLE_RATE_HZ as usize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Refinement {
    #[default]
    Works,
    Off,
    Fails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Pasted(String),
    Hidden,
}

type Events = Arc<Mutex<Vec<Event>>>;
/// The audio of each recording still to come, part by part.
type Takes = Arc<Mutex<VecDeque<Vec<AudioBuffer16k>>>>;
/// What speech recognition answers, call by call.
type Answers = Arc<Mutex<VecDeque<String>>>;

/// What a dictation-archive scenario shares with the pipeline thread.
#[derive(Default)]
pub struct ArchiveRun {
    folder: Option<TempDir>,
    unwritable: bool,
    refinement: Refinement,
    recognition_fails: bool,
    voice_control: bool,
    in_chunks: bool,
    /// Recordings shorter than this are dropped as too short.
    min_press: Duration,
    events: Events,
    takes: Takes,
    answers: Answers,
    inputs: Option<Sender<PipelineInput>>,
    /// The thread the pipeline runs on: what it logs is the scenario's.
    pipeline_thread: Option<ThreadId>,
    /// The audio of each dictation, in the order dictated.
    dictated: Vec<AudioBuffer16k>,
}

impl std::fmt::Debug for ArchiveRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchiveRun")
            .field("events", &self.events())
            .finish()
    }
}

impl ArchiveRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn archive_dir(&self) -> Result<PathBuf> {
        let folder = self.folder.as_ref().context("the archive is not set up")?;
        Ok(folder.path().join("archive"))
    }

    fn turn_archive_on(&mut self) -> Result<()> {
        self.folder = Some(TempDir::new()?);
        Ok(())
    }

    /// The archive folder's place holds a file, so no folder can be made there.
    fn make_archive_unwritable(&mut self) -> Result<()> {
        self.turn_archive_on()?;
        std::fs::write(self.archive_dir()?, b"not a folder")?;
        self.unwritable = true;
        Ok(())
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline()?);
        }
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    fn start_pipeline(&mut self) -> Result<Sender<PipelineInput>> {
        pipeline_log();
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            Box::new(TakesRecorder {
                takes: Arc::clone(&self.takes),
                in_chunks: self.in_chunks,
                current: Vec::new(),
                parts: None,
            }),
            Box::new(AnsweringTranscriber {
                answers: Arc::clone(&self.answers),
                fails: self.recognition_fails,
            }),
            self.refiner(),
            Box::new(NotingPaster(Arc::clone(&self.events))),
            Box::new(Silent),
            Box::new(NotingOverlay(Arc::clone(&self.events))),
            PipelineConfig {
                min_press_ms: self.min_press.as_millis() as u64,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: if self.in_chunks { 5 } else { 0 },
                    ..ChunkingCfg::default()
                },
            },
        )
        .with_archive(Box::new(FileSystemDictationArchive::new(
            self.archive_dir()?,
            ARCHIVE_SIZE_LIMIT_BYTES,
            archive_setup(),
        )));
        if self.refinement == Refinement::Off {
            pipeline = pipeline.without_text_refinement_stage();
        }
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        let thread = std::thread::spawn(move || pipeline.run(rx));
        self.pipeline_thread = Some(thread.thread().id());
        Ok(tx)
    }

    fn refiner(&self) -> Box<dyn TextRefiner> {
        match self.refinement {
            Refinement::Works => Box::new(SentenceRefiner),
            Refinement::Off => Box::new(PassthroughTextRefiner),
            Refinement::Fails => Box::new(FailingRefiner),
        }
    }

    /// Queues the next recording's audio, `parts` seconds in a distinct sound, and what
    /// speech recognition answers for it.
    fn queue_dictation(&mut self, parts: usize, answers: &[&str]) {
        let number = self.dictated.len() as i16 + 1;
        let audio: Vec<AudioBuffer16k> = (0..parts as i16)
            .map(|part| AudioBuffer16k::new(vec![number * 1_000 + part; SAMPLES_PER_PART]))
            .collect();
        self.dictated.push(joined(&audio));
        self.takes.lock().unwrap().push_back(audio);
        let mut queued = self.answers.lock().unwrap();
        queued.extend(answers.iter().map(|answer| (*answer).to_owned()));
    }

    /// Dictates with the hotkey: one press starts the recording, the next ends it.
    fn dictate_with_hotkey(&mut self, parts: usize, answers: &[&str]) -> Result<()> {
        self.queue_dictation(parts, answers);
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts and stops a dictation by voice, each command said with `placement`.
    fn dictate_by_voice(&mut self, answer: &str, placement: CommandPlacement) -> Result<()> {
        self.queue_dictation(1, &[answer]);
        let said_at = Instant::now()
            .checked_sub(COMMANDS_SAID_AGO)
            .context("the clock started too recently")?;
        self.say(VoiceCommand::StartDictation, said_at, placement)?;
        self.say(VoiceCommand::StopDictation, said_at + STOP_AFTER, placement)
    }

    /// Starts a dictation by voice, drops it with "restart dictation", then stops the
    /// restarted one by voice: speech recognition only ever hears the restarted one.
    fn dictate_after_a_restart(&mut self) -> Result<()> {
        self.queue_dictation(1, &[]);
        self.queue_dictation(1, &[PHRASE]);
        let said_at = Instant::now()
            .checked_sub(COMMANDS_SAID_AGO)
            .context("the clock started too recently")?;
        let alone = CommandPlacement::Alone;
        self.say(VoiceCommand::StartDictation, said_at, alone)?;
        self.say(
            VoiceCommand::RestartDictation,
            said_at + STOP_AFTER / 2,
            alone,
        )?;
        self.say(VoiceCommand::StopDictation, said_at + STOP_AFTER, alone)
    }

    fn say(
        &mut self,
        command: VoiceCommand,
        at: Instant,
        placement: CommandPlacement,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::CommandsSaid {
            commands: vec![command],
            start: at,
            end: at + COMMAND_LENGTH,
            placement,
            timing: None,
        }))
    }

    fn wait_until(&self, what: &str, done: impl Fn(&Self) -> Result<bool>) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !done(self)? {
            if Instant::now() >= deadline {
                bail!(
                    "{what} not seen within {REACTION:?}; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(POLL);
        }
        Ok(())
    }

    /// The archive entries once `count` of them are written, oldest first.
    fn entries(&self, count: usize) -> Result<Vec<ReadEntry>> {
        let dir = self.archive_dir()?;
        self.wait_until(&format!("{count} archive entries"), |_| {
            Ok(entry_folders(&dir)?.len() >= count)
        })?;
        std::thread::sleep(SETTLE);
        let folders = entry_folders(&dir)?;
        if folders.len() != count {
            bail!("expected {count} archive entries, found {folders:?}");
        }
        folders
            .iter()
            .map(|folder| ReadEntry::read(folder))
            .collect()
    }

    fn only_entry(&self) -> Result<ReadEntry> {
        self.entries(1)?.pop().context("no archive entry")
    }

    fn pasted(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Pasted(text) => Some(text),
                Event::Hidden => None,
            })
            .collect()
    }

    fn warnings(&self) -> Vec<String> {
        let thread = self.pipeline_thread;
        pipeline_log()
            .warnings
            .lock()
            .unwrap()
            .iter()
            .filter(|(logged_by, _)| Some(*logged_by) == thread)
            .map(|(_, line)| line.clone())
            .collect()
    }

    fn dictated(&self, number: usize) -> Result<&AudioBuffer16k> {
        self.dictated
            .get(number)
            .with_context(|| format!("no dictation {number}"))
    }
}

/// One archive entry as read back from its folder.
#[derive(Debug)]
struct ReadEntry {
    json: serde_json::Value,
    samples: Vec<i16>,
}

impl ReadEntry {
    fn read(folder: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(folder.join(ENTRY_FILE))?;
        let json = serde_json::from_str(&text)?;
        let samples = hound::WavReader::open(folder.join(AUDIO_FILE))?
            .samples::<i16>()
            .collect::<Result<_, _>>()?;
        Ok(Self { json, samples })
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.json[key].as_str()
    }

    fn expect(&self, key: &str, expected: impl Into<serde_json::Value>) -> Result<()> {
        let expected = expected.into();
        if self.json[key] != expected {
            bail!(
                "entry {key} is {}, expected {expected}: {}",
                self.json[key],
                self.json
            );
        }
        Ok(())
    }

    fn expect_audio(&self, audio: &AudioBuffer16k) -> Result<()> {
        if self.samples != audio.samples() {
            bail!(
                "entry audio has {} samples, not the {} dictated",
                self.samples.len(),
                audio.samples().len()
            );
        }
        self.expect(
            "audio",
            serde_json::json!({"file": AUDIO_FILE, "duration_ms": audio.duration_ms()}),
        )
    }

    fn expect_pasted_raw_transcript(&self) -> Result<()> {
        let raw = self.text("raw_transcript").context("no raw transcript")?;
        self.expect("pasted_text", raw)
    }
}

/// The folders of written entries in `dir`, sorted by name.
fn entry_folders(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut folders: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    folders.retain(|folder| folder.join(ENTRY_FILE).is_file());
    folders.sort();
    Ok(folders)
}

fn archive_setup() -> ArchiveSetup {
    ArchiveSetup {
        dictation_model: DICTATION_MODEL.to_owned(),
        refinement_model: Some(REFINEMENT_MODEL.to_owned()),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// The text refinement service's answer: the phrase as a sentence.
fn as_sentence(raw: &str) -> String {
    let mut chars = raw.chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
    format!("{}{}.", first.unwrap_or_default(), chars.as_str())
}

fn joined(parts: &[AudioBuffer16k]) -> AudioBuffer16k {
    AudioBuffer16k::new(
        parts
            .iter()
            .flat_map(|part| part.samples().to_vec())
            .collect(),
    )
}

fn run(world: &mut IntegrationWorld) -> &mut ArchiveRun {
    world.archive.get_or_insert_with(ArchiveRun::default)
}

#[given("the archive is on")]
fn archive_on(world: &mut IntegrationWorld) -> Result<()> {
    run(world).turn_archive_on()
}

#[given("the archive is on and text refinement is turned off")]
fn archive_on_refinement_off(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.refinement = Refinement::Off;
    run.turn_archive_on()
}

#[given("the archive is on and the text refinement service fails")]
fn archive_on_refinement_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.refinement = Refinement::Fails;
    run.turn_archive_on()
}

#[given("the archive is on and the speech recognition service fails")]
fn archive_on_recognition_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recognition_fails = true;
    run.turn_archive_on()
}

#[given("the archive is on and voice control is on")]
fn archive_on_voice_control(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.turn_archive_on()
}

#[given("an archive folder the app cannot write to")]
fn archive_unwritable(world: &mut IntegrationWorld) -> Result<()> {
    run(world).make_archive_unwritable()
}

#[when("the user dictates a phrase with the hotkey and it is pasted after text refinement")]
#[when("the user dictates a phrase")]
fn dictates_phrase(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_with_hotkey(1, &[PHRASE])
}

#[when("the user dictates long enough for the dictation to be transcribed in several chunks")]
fn dictates_in_chunks(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.in_chunks = true;
    run.dictate_with_hotkey(CHUNK_TEXTS.len(), &CHUNK_TEXTS)
}

#[when("the user dictates two phrases one after the other")]
fn dictates_two_phrases(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.dictate_with_hotkey(1, &[PHRASE])?;
    run.entries(1)?;
    run.dictate_with_hotkey(2, &[SECOND_PHRASE])
}

#[when("the user starts a dictation by voice and stops it without saying anything else")]
fn voice_dictation_without_words(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_by_voice(ONLY_COMMAND_WORDS, CommandPlacement::Tail)
}

#[when("the user starts a dictation by voice, dictates a phrase and stops it by voice")]
fn voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_by_voice(PHRASE, CommandPlacement::Alone)
}

#[when("the user cancels a dictation")]
fn cancels_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.queue_dictation(1, &[PHRASE]);
    run.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when(
    "the user starts a dictation by voice, restarts it, dictates a phrase and stops it by voice"
)]
fn restarted_voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_after_a_restart()
}

#[when("the user records for less than the minimum press time")]
fn too_short_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.min_press = MIN_PRESS;
    run.dictate_with_hotkey(1, &[PHRASE])
}

#[then("the archive holds one entry, with the audio of the restarted dictation only")]
fn restarted_dictation_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    std::thread::sleep(SETTLE);
    let folders = entry_folders(&run.archive_dir()?)?;
    if folders.len() != 1 {
        bail!("expected only the restarted dictation's entry, found {folders:?}");
    }
    entry.expect_audio(run.dictated(1)?)?;
    entry.expect("start", "voice")
}

#[then(
    "a new archive entry holds the audio of the dictation, the raw transcript speech recognition returned, and the refined text that was pasted"
)]
fn entry_with_audio_and_texts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("raw_transcript", PHRASE)?;
    entry.expect("pasted_text", as_sentence(PHRASE))?;
    entry.expect("refinement", "refined")?;
    entry.expect("outcome", "pasted")?;
    entry.expect("start", "hotkey_toggled")
}

#[then(
    "its archive entry holds the text of each chunk in the order spoken, the stitched raw transcript and the audio of the whole dictation"
)]
fn entry_with_every_chunk(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect("chunks", CHUNK_TEXTS.to_vec())?;
    entry.expect("raw_transcript", STITCHED_CHUNKS)?;
    entry.expect_audio(run.dictated(0)?)
}

#[then(
    "the archive holds two entries, each with only its own audio and texts, the later one sorting after the earlier one"
)]
fn two_entries(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entries = run.entries(2)?;
    for (number, (entry, phrase)) in entries.iter().zip([PHRASE, SECOND_PHRASE]).enumerate() {
        entry.expect_audio(run.dictated(number)?)?;
        entry.expect("chunks", vec![phrase])?;
        entry.expect("raw_transcript", phrase)?;
        entry.expect("pasted_text", as_sentence(phrase))?;
    }
    Ok(())
}

#[then("its entry says refinement was off and the pasted text equals the raw transcript")]
fn refinement_off_entry(world: &mut IntegrationWorld) -> Result<()> {
    let entry = run(world).only_entry()?;
    entry.expect("refinement", "off")?;
    entry.expect_pasted_raw_transcript()
}

#[then("its entry says refinement fell back and the pasted text equals the raw transcript")]
fn refinement_fell_back_entry(world: &mut IntegrationWorld) -> Result<()> {
    let entry = run(world).only_entry()?;
    entry.expect("refinement", "fell_back")?;
    entry.expect_pasted_raw_transcript()
}

#[then("its entry holds the audio and says speech recognition failed, with no pasted text")]
fn recognition_failed_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("outcome", "speech_recognition_failed")?;
    entry.expect("pasted_text", serde_json::Value::Null)
}

#[then("its entry holds the audio and says only command words were transcribed")]
fn only_command_words_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("outcome", "only_command_words")?;
    entry.expect("pasted_text", serde_json::Value::Null)
}

#[then("the archive holds no entry for it")]
fn no_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until("the overlay hiding", |run| {
        Ok(run.events().contains(&Event::Hidden))
    })?;
    std::thread::sleep(SETTLE);
    let folders = entry_folders(&run.archive_dir()?)?;
    if !folders.is_empty() {
        bail!("a dropped dictation left entries: {folders:?}");
    }
    Ok(())
}

#[then("its entry says it started by voice and names the app version and the models")]
fn voice_started_entry(world: &mut IntegrationWorld) -> Result<()> {
    let entry = run(world).only_entry()?;
    entry.expect("start", "voice")?;
    entry.expect("app_version", env!("CARGO_PKG_VERSION"))?;
    entry.expect("dictation_model", DICTATION_MODEL)?;
    entry.expect("refinement_model", REFINEMENT_MODEL)?;
    entry.expect("outcome", "pasted")
}

#[then("the phrase is pasted as usual and the log warns that the dictation was not archived")]
fn pasted_and_warned(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until("the warning", |run| {
        Ok(run
            .warnings()
            .iter()
            .any(|line| line.starts_with(NOT_ARCHIVED_WARNING)))
    })?;
    if run.pasted() != [as_sentence(PHRASE)] {
        bail!("the phrase was not pasted as usual: {:?}", run.events());
    }
    if !run.unwritable {
        bail!("the archive folder was writable");
    }
    Ok(())
}

/// Hands over the audio queued for each recording; recorded in chunks, it sends all parts
/// but the last while the user speaks and the last one when the recording is flushed.
struct TakesRecorder {
    takes: Takes,
    in_chunks: bool,
    current: Vec<AudioBuffer16k>,
    parts: Option<Sender<LiveChunk>>,
}

impl TakesRecorder {
    fn send(&self, audio: &AudioBuffer16k) {
        if let Some(parts) = &self.parts {
            let _ = parts.send(LiveChunk {
                audio: audio.clone(),
                enqueued_at: Instant::now(),
            });
        }
    }
}

impl AudioRecorder for TakesRecorder {
    fn start(&mut self) -> Result<()> {
        self.current = self.takes.lock().unwrap().pop_front().unwrap_or_default();
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        self.parts = None;
        Ok(joined(&self.current))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.in_chunks {
            return Ok(None);
        }
        let (tx, rx) = mpsc::channel();
        self.parts = Some(tx);
        let spoken = self.current.len().saturating_sub(1);
        for part in &self.current[..spoken] {
            self.send(part);
        }
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        if let Some(last) = self.current.last() {
            self.send(last);
        }
        // the transcription of the parts ends with the last one
        self.parts = None;
    }

    fn recording(&mut self) -> Option<AudioBuffer16k> {
        Some(joined(&self.current))
    }
}

struct AnsweringTranscriber {
    answers: Answers,
    fails: bool,
}

impl Transcriber for AnsweringTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let answer = self.answers.lock().unwrap().pop_front();
        if self.fails {
            return Err(anyhow!("the speech recognition service does not answer"));
        }
        Ok(answer.map(RawTranscript::new).transpose()?)
    }
}

struct SentenceRefiner;

impl TextRefiner for SentenceRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        Ok(CleanedText::new(as_sentence(raw.as_str()))?)
    }
}

struct FailingRefiner;

impl TextRefiner for FailingRefiner {
    fn refine(&mut self, _raw: &RawTranscript) -> Result<CleanedText> {
        Err(anyhow!("the text refinement service does not answer"))
    }
}

struct NotingPaster(Events);

impl TextPaster for NotingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .push(Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

struct NotingOverlay(Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}

    fn hide(&self) {
        self.0.lock().unwrap().push(Event::Hidden);
    }

    /// The Pasted sign takes itself down: the dictation ends on the overlay with it.
    fn show_pasted(&self) {
        self.hide();
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        self.hide();
    }
}

/// The command spotting service as the pipeline sees it; the scenario reports the commands
/// straight into the pipeline's input channel.
struct QuietSpotter;

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
}

struct NoKeys;

impl KeyPresser for NoKeys {
    fn press_enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        Ok(())
    }
}

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}
