//! The archive run: the pipeline writing into a temporary archive, and reading its entries back.

use super::{
    bail, limit, mpsc, pipeline_log, AnsweringTranscriber, Arc, AudioBuffer16k, ChunkingCfg,
    CommandPlacement, Context, Duration, Event, Instant, IntegrationWorld, Mutex, NoKeys,
    NotingOverlay, NotingPaster, Path, PathBuf, Pipeline, PipelineConfig, PipelineInput,
    PipelineParts, PttSignal, QuietSpotter, Result, Sender, Silent, SpottingEvent, TakesRecorder,
    TempDir, ThreadId, VecDeque, VoiceCommand, ARCHIVE_SIZE_LIMIT_BYTES, AUDIO_FILE,
    COMMANDS_SAID_AGO, COMMAND_LENGTH, ENTRY_FILE, PHRASE, POLL, REACTION, SAMPLES_PER_PART,
    SETTLE, STOP_AFTER,
};

pub(super) type Events = Arc<Mutex<Vec<Event>>>;

/// The audio of each recording still to come, part by part.
pub(super) type Takes = Arc<Mutex<VecDeque<Vec<AudioBuffer16k>>>>;

/// What speech recognition answers, call by call.
pub(super) type Answers = Arc<Mutex<VecDeque<String>>>;

/// What a dictation-archive scenario shares with the pipeline thread.
#[derive(Default)]
pub struct ArchiveRun {
    folder: Option<TempDir>,
    pub(crate) recognition_fails: bool,
    pub(crate) voice_control: bool,
    pub(crate) in_chunks: bool,
    /// Recordings shorter than this are dropped as too short.
    pub(crate) min_press: Duration,
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
    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub(crate) fn archive_dir(&self) -> Result<PathBuf> {
        let folder = self.folder.as_ref().context("the archive is not set up")?;
        Ok(folder.path().join("archive"))
    }

    pub(crate) fn turn_archive_on(&mut self) -> Result<()> {
        self.folder = Some(TempDir::new()?);
        Ok(())
    }

    /// The archive folder's place holds a file, so no folder can be made there.
    pub(crate) fn make_archive_unwritable(&mut self) -> Result<()> {
        self.turn_archive_on()?;
        std::fs::write(self.archive_dir()?, b"not a folder")?;
        Ok(())
    }

    /// Whether a file holds the archive folder's place.
    pub(crate) fn is_unwritable(&self) -> bool {
        self.archive_dir().is_ok_and(|dir| dir.is_file())
    }

    pub(crate) fn send(&mut self, input: PipelineInput) -> Result<()> {
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
            self.parts(),
            PipelineConfig {
                min_press_ms: u64::try_from(self.min_press.as_millis()).unwrap(),
                chunking: ChunkingCfg {
                    chunk_threshold_secs: if self.in_chunks { 5 } else { 0 },
                    ..ChunkingCfg::default()
                },
                ..PipelineConfig::default()
            },
        )
        .with_archive(Box::new(limit::archive(
            self.archive_dir()?,
            ARCHIVE_SIZE_LIMIT_BYTES,
        )));
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        let thread = std::thread::spawn(move || pipeline.run(rx));
        self.pipeline_thread = Some(thread.thread().id());
        Ok(tx)
    }

    fn parts(&self) -> PipelineParts {
        PipelineParts {
            recorder: Box::new(TakesRecorder {
                takes: Arc::clone(&self.takes),
                in_chunks: self.in_chunks,
                current: Vec::new(),
                parts: None,
            }),
            transcriber: Box::new(AnsweringTranscriber {
                answers: Arc::clone(&self.answers),
                fails: self.recognition_fails,
            }),
            paster: Box::new(NotingPaster::new(&self.events, Event::Pasted)),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(NotingOverlay(Arc::clone(&self.events))),
        }
    }

    /// Queues the next recording's audio, `parts` seconds in a distinct sound, and what
    /// speech recognition answers for it.
    pub(crate) fn queue_dictation(&mut self, parts: usize, answers: &[&str]) {
        let number = i16::try_from(self.dictated.len()).unwrap() + 1;
        let audio: Vec<AudioBuffer16k> = (0..i16::try_from(parts).unwrap())
            .map(|part| AudioBuffer16k::new(vec![number * 1_000 + part; SAMPLES_PER_PART]))
            .collect();
        self.dictated.push(joined(&audio));
        self.takes.lock().unwrap().push_back(audio);
        let mut queued = self.answers.lock().unwrap();
        queued.extend(answers.iter().map(|answer| (*answer).to_owned()));
    }

    /// Dictates with the hotkey: one press starts the recording, the next ends it.
    pub(crate) fn dictate_with_hotkey(&mut self, parts: usize, answers: &[&str]) -> Result<()> {
        self.queue_dictation(parts, answers);
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts and stops a dictation by voice, each command said with `placement`.
    pub(crate) fn dictate_by_voice(
        &mut self,
        answer: &str,
        placement: CommandPlacement,
    ) -> Result<()> {
        self.queue_dictation(1, &[answer]);
        let said_at = Instant::now()
            .checked_sub(COMMANDS_SAID_AGO)
            .context("the clock started too recently")?;
        self.say(VoiceCommand::StartDictation, said_at, placement)?;
        self.say(VoiceCommand::StopDictation, said_at + STOP_AFTER, placement)
    }

    /// Starts a dictation by voice, drops it with "restart dictation", then stops the
    /// restarted one by voice: speech recognition only ever hears the restarted one.
    pub(crate) fn dictate_after_a_restart(&mut self) -> Result<()> {
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

    pub(crate) fn wait_until(
        &self,
        what: &str,
        done: impl Fn(&Self) -> Result<bool>,
    ) -> Result<()> {
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
    pub(crate) fn entries(&self, count: usize) -> Result<Vec<ReadEntry>> {
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

    pub(crate) fn only_entry(&self) -> Result<ReadEntry> {
        self.entries(1)?.pop().context("no archive entry")
    }

    pub(crate) fn pasted(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Pasted(text) => Some(text),
                Event::Hidden => None,
            })
            .collect()
    }

    pub(crate) fn warnings(&self) -> Vec<String> {
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

    pub(crate) fn dictated(&self, number: usize) -> Result<&AudioBuffer16k> {
        self.dictated
            .get(number)
            .with_context(|| format!("no dictation {number}"))
    }
}

/// One archive entry as read back from its folder.
#[derive(Debug)]
pub(crate) struct ReadEntry {
    pub(crate) json: serde_json::Value,
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

    pub(crate) fn expect(&self, key: &str, expected: impl Into<serde_json::Value>) -> Result<()> {
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

    pub(crate) fn expect_audio(&self, audio: &AudioBuffer16k) -> Result<()> {
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

    pub(crate) fn expect_pasted_raw_transcript(&self) -> Result<()> {
        let raw = self.text("raw_transcript").context("no raw transcript")?;
        self.expect("pasted_text", raw)
    }
}

/// The folders of written entries in `dir`, sorted by name.
pub(super) fn entry_folders(dir: &Path) -> Result<Vec<PathBuf>> {
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

pub(super) fn joined(parts: &[AudioBuffer16k]) -> AudioBuffer16k {
    AudioBuffer16k::new(
        parts
            .iter()
            .flat_map(|part| part.samples().to_vec())
            .collect(),
    )
}

pub(super) fn run(world: &mut IntegrationWorld) -> &mut ArchiveRun {
    world
        .pipeline
        .archive
        .get_or_insert_with(ArchiveRun::default)
}
