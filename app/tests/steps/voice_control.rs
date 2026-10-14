//! Drives the real pipeline on its own thread with voice control: a scenario speaks by
//! sending the speech and phrase events the command spotting service would report, and
//! watches the ordered log of what the stand-ins in `doubles` were asked to do.

mod doubles;
mod speech_after_start;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{ChunkingCfg, PipelineInput};
use voicepuncher::core::domain::{
    CommandPlacement, PttSignal, RecordingMode, SpottingEvent, VoiceCommand,
};

use doubles::{
    Cut, ErrorLog, Event, Events, NotingRecorder, Phrases, RecordingKeyPresser, RecordingOverlay,
    RecordingPaster, RecordingSounds, ScriptedSpotter, Speech, SpeechBeforeCut,
};

use crate::IntegrationWorld;

const SENTENCE: &str = "Meeting moved to Friday.";
/// Said in the same breath after a doubled command: none of it may be pasted.
const MORE_WORDS: &str = "and some more words";
/// Where a command said without a pause sits after the start of its utterance.
const COMMAND_AFTER: Duration = Duration::from_millis(1_500);
/// How long a command's words last.
const COMMAND_LENGTH: Duration = Duration::from_millis(500);
/// How late the service places a command's start: a cut this late keeps the first command
/// word in the recording, so the text must lose it.
const LATE_CUT: Duration = Duration::from_millis(50);
const STOP_PHRASE: &str = "stop dictation";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(2);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// Where a phrase spoken on its own sits in its speech segment.
const PHRASE_LEAD: Duration = Duration::from_millis(100);
const PHRASE_LENGTH: Duration = Duration::from_millis(800);
const SEGMENT_TAIL: Duration = Duration::from_millis(100);
/// How much speech surrounds a phrase said inside a sentence.
const SENTENCE_AROUND_PHRASE: Duration = Duration::from_secs(1);

static ERRORS: OnceLock<&'static ErrorLog> = OnceLock::new();

pub(crate) fn error_log() -> &'static ErrorLog {
    ERRORS.get_or_init(|| {
        let errors: &'static ErrorLog = Box::leak(Box::new(ErrorLog(Mutex::default())));
        if log::set_logger(errors).is_ok() {
            log::set_max_level(log::LevelFilter::Error);
        }
        errors
    })
}

/// What a voice-control scenario shares with the pipeline thread.
#[derive(Default)]
pub struct VoiceRun {
    voice_control: bool,
    events: Events,
    speech: Speech,
    cut: Cut,
    phrases: Phrases,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the scenario's `When` started.
    mark: usize,
    /// When the command spotting service went away, if it did.
    away_since: Option<Instant>,
}

impl std::fmt::Debug for VoiceRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceRun")
            .field("events", &self.events())
            .finish()
    }
}

impl VoiceRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn since_mark(&self) -> Vec<Event> {
        self.events()[self.mark..].to_vec()
    }

    fn start_when(&mut self) {
        self.mark = self.events.lock().unwrap().len();
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    /// Starts the pipeline unless it already runs.
    fn ensure_started(&mut self) {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
    }

    fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts a dictation with the hotkey and says the sentence into it.
    fn dictate_with_hotkey(&mut self) -> Result<()> {
        let started = self.events().len();
        self.press_hotkey()?;
        let deadline = Instant::now() + REACTION;
        while !self.events()[started..].contains(&Event::RecordingStarted) {
            if Instant::now() >= deadline {
                bail!(
                    "the hotkey started no dictation; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.speak(SENTENCE, Instant::now());
        Ok(())
    }

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        error_log();
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            Box::new(NotingRecorder {
                events: Arc::clone(&self.events),
                speech: Arc::clone(&self.speech),
                cut: Arc::clone(&self.cut),
                said_before: Vec::new(),
            }),
            Box::new(SpeechBeforeCut {
                speech: Arc::clone(&self.speech),
                cut: Arc::clone(&self.cut),
            }),
            Box::new(PassthroughTextRefiner),
            Box::new(RecordingPaster(Arc::clone(&self.events))),
            Box::new(RecordingSounds(Arc::clone(&self.events))),
            Box::new(RecordingOverlay(Arc::clone(&self.events))),
            PipelineConfig {
                min_press_ms: 0,
                recording_mode: RecordingMode::Toggle,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: 0,
                    ..ChunkingCfg::default()
                },
            },
        );
        if self.voice_control {
            pipeline = pipeline.with_voice_control(
                Box::new(ScriptedSpotter {
                    events: Arc::clone(&self.events),
                    phrases: Arc::clone(&self.phrases),
                }),
                Box::new(RecordingKeyPresser(Arc::clone(&self.events))),
            );
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    /// Says `words` as a whole utterance, with a pause before and after.
    fn say_alone(&mut self, words: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + PHRASE_LEAD;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(words, phrase_start);
        self.send_segment(
            words,
            segment_start,
            phrase_start,
            phrase_end,
            phrase_end + SEGMENT_TAIL,
        )
    }

    /// Says `sentence`, which holds a command phrase, without pausing around the phrase.
    fn say_in_sentence(&mut self, sentence: &str, phrase: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + SENTENCE_AROUND_PHRASE;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(sentence, segment_start);
        let segment_end = phrase_end + SENTENCE_AROUND_PHRASE;
        self.send_segment(phrase, segment_start, phrase_start, phrase_end, segment_end)
    }

    /// Reports `commands` said without a pause at `start..end`, as the service does.
    fn say_commands(
        &mut self,
        commands: &[VoiceCommand],
        start: Instant,
        end: Instant,
        placement: CommandPlacement,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::CommandsSaid {
            commands: commands.to_vec(),
            start,
            end,
            placement,
        }))
    }

    fn speak(&self, words: &str, at: Instant) {
        self.speech.lock().unwrap().push((at, words.to_owned()));
    }

    fn send_segment(
        &mut self,
        phrase: &str,
        segment_start: Instant,
        start: Instant,
        end: Instant,
        segment_end: Instant,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment_start,
        )))?;
        if self.listened_for(phrase) {
            let phrase = phrase.to_owned();
            self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
                phrase,
                start,
                end,
            }))?;
        }
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            segment_end,
        )))
    }

    /// Whether the service listens for `phrase` now. The pipeline may still be switching
    /// the phrase set after an earlier input, so this waits for it to settle first.
    fn listened_for(&self, phrase: &str) -> bool {
        std::thread::sleep(Duration::from_millis(20));
        VoiceCommand::from_phrase(phrase)
            .is_some_and(|command| self.phrases.lock().unwrap().contains(&command))
    }

    fn wait_for(&self, wanted: impl Fn(&Event) -> bool) -> Result<usize> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(at) = self.events().iter().position(&wanted) {
                return Ok(at);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits until `wanted` happens after the scenario's `When` started.
    fn wait_since_mark(&self, wanted: impl Fn(&Event) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.since_mark().iter().any(&wanted) {
            if Instant::now() >= deadline {
                bail!(
                    "not seen after the action within {REACTION:?}; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    fn never_since_mark(&self, unwanted: impl Fn(&Event) -> bool, what: &str) -> Result<()> {
        std::thread::sleep(SETTLE);
        if let Some(event) = self.since_mark().into_iter().find(unwanted) {
            bail!("{what}: {event:?}; events: {:?}", self.events());
        }
        Ok(())
    }

    fn listening(&self) -> bool {
        self.events()
            .iter()
            .rev()
            .find(|e| matches!(e, Event::ListeningStarted | Event::ListeningStopped))
            == Some(&Event::ListeningStarted)
    }

    fn pasted(&self) -> Result<String> {
        let at = self.wait_for(|e| matches!(e, Event::Pasted(_)))?;
        match &self.events()[at] {
            Event::Pasted(text) => Ok(text.clone()),
            other => bail!("expected a paste, got {other:?}"),
        }
    }
}

fn run(world: &mut IntegrationWorld) -> &mut VoiceRun {
    world.voice.get_or_insert_with(VoiceRun::default)
}

fn is_key_press(e: &Event) -> bool {
    matches!(e, Event::ReturnPressed | Event::AllTextCleared)
}

#[given("voice control is not turned on")]
fn voice_control_off(world: &mut IntegrationWorld) {
    run(world).voice_control = false;
}

#[given("voice control is turned on")]
fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("no dictation runs")]
fn no_dictation_given(_world: &mut IntegrationWorld) {}

#[given("listening is on")]
fn listening_on_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    Ok(())
}

#[given("a dictation runs")]
fn dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    listening_on_given(world)?;
    let run = run(world);
    run.say_alone("start dictation")?;
    run.wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[given("the user started a dictation with the hotkey and said a sentence")]
fn hotkey_dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    listening_on_given(world)?;
    run(world).dictate_with_hotkey()
}

#[given("the user has said a sentence in it")]
fn sentence_said(world: &mut IntegrationWorld) {
    run(world).speak(SENTENCE, Instant::now());
}

#[given(expr = "the user has said {string}")]
fn phrase_said(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.speak(SENTENCE, Instant::now());
    run.say_alone(&phrase)
}

#[when("the app starts")]
fn app_starts(world: &mut IntegrationWorld) {
    let run = run(world);
    run.start_when();
    run.inputs = Some(run.start_pipeline());
}

#[when("the user presses the hotkey once")]
fn press_hotkey_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()
}

#[when(expr = "the user says a sentence and then {string} without a pause, then pauses")]
fn says_sentence_then_command(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    let start = Instant::now();
    let command_at = start + COMMAND_AFTER;
    run.speak(SENTENCE, start);
    run.speak(&said, command_at);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = chain(&said).context("no command")?;
    let end = command_at + COMMAND_LENGTH;
    run.say_commands(
        &commands,
        command_at + LATE_CUT,
        end,
        CommandPlacement::Tail,
    )?;
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(end)))
}

#[when(expr = "the user says a sentence, {string} and more words, without any pause")]
fn says_doubled_in_speech(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    let start = Instant::now();
    let command_at = start + COMMAND_AFTER;
    let end = command_at + COMMAND_LENGTH;
    run.speak(SENTENCE, start);
    run.speak(&said, command_at);
    run.speak(MORE_WORDS, end);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = doubled(&said).context("no doubled command")?;
    run.say_commands(
        &commands,
        command_at + LATE_CUT,
        end,
        CommandPlacement::Anywhere,
    )
}

#[when(expr = "the user says a sentence and {string} without a pause")]
fn says_sentence_and_doubled(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    let start = Instant::now();
    let command_at = start + COMMAND_AFTER;
    run.speak(SENTENCE, start);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = doubled(&said).context("no doubled command")?;
    run.say_commands(
        &commands,
        command_at,
        command_at + COMMAND_LENGTH,
        CommandPlacement::Anywhere,
    )
}

#[when("the user presses Escape")]
fn press_escape(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when("the user records a dictation with the hotkey")]
fn records_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.dictate_with_hotkey()?;
    run.press_hotkey()?;
    run.pasted()?;
    Ok(())
}

#[when(expr = "the user says {string}")]
#[when(expr = "the user says {string} before the text is pasted")]
fn says(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    match chain(&phrase) {
        Some(commands) if commands.len() > 1 => {
            let start = Instant::now();
            run.say_commands(
                &commands,
                start,
                start + COMMAND_LENGTH,
                CommandPlacement::Anywhere,
            )
        }
        Some(commands) if commands.len() == 1 => run.say_alone(commands[0].phrase()),
        _ => run.say_alone(&phrase),
    }
}

/// The command a short form names.
fn short_form(word: &str) -> Option<VoiceCommand> {
    match word {
        "start" => Some(VoiceCommand::StartDictation),
        "stop" => Some(VoiceCommand::StopDictation),
        "cancel" => Some(VoiceCommand::CancelDictation),
        "restart" => Some(VoiceCommand::RestartDictation),
        "enter" => Some(VoiceCommand::PressEnter),
        "clear" => Some(VoiceCommand::ClearAllText),
        _ => None,
    }
}

/// The commands of a chain of short forms ("stop enter start").
fn chain(said: &str) -> Option<Vec<VoiceCommand>> {
    said.split_whitespace().map(short_form).collect()
}

/// The command a doubled short form ("stop stop") names.
fn doubled(said: &str) -> Option<Vec<VoiceCommand>> {
    match chain(said)?.as_slice() {
        [first, second] if first == second => Some(vec![*first]),
        _ => None,
    }
}

#[when(expr = "the user says {string} without pausing")]
fn says_in_sentence(world: &mut IntegrationWorld, sentence: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_in_sentence(&sentence, STOP_PHRASE)
}

#[when("the command spotting service stops answering")]
fn service_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::SpottingLost)
}

#[given("the command spotting service went away during it")]
fn service_went_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.away_since = Some(Instant::now());
    run.send(PipelineInput::SpottingLost)?;
    run.wait_for(|e| *e == Event::Sound("listening off"))?;
    Ok(())
}

#[given("the command spotting service went away and is back")]
#[given("the command spotting service went away during it and is back")]
fn service_went_away_and_is_back(world: &mut IntegrationWorld) -> Result<()> {
    service_went_away(world)?;
    let run = run(world);
    run.send(PipelineInput::SpottingBack)?;
    run.wait_for(|e| {
        *e == Event::Sound("listening on") && run.events().contains(&Event::Sound("listening off"))
    })?;
    std::thread::sleep(SETTLE);
    Ok(())
}

#[when(expr = "the service is back and reports {string}, said while it was away")]
fn back_with_command_said_away(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    let away_since = run.away_since.context("the service did not go away")?;
    run.start_when();
    run.send(PipelineInput::SpottingBack)?;
    run.wait_since_mark(|e| *e == Event::Sound("listening on"))?;
    let segment_start = away_since + PHRASE_LEAD;
    let phrase_start = segment_start + PHRASE_LEAD;
    let phrase_end = phrase_start + PHRASE_LENGTH;
    run.speak(&phrase, phrase_start);
    run.send_segment(
        &phrase,
        segment_start,
        phrase_start,
        phrase_end,
        phrase_end + SEGMENT_TAIL,
    )
}

#[then("nothing was pasted before the dictation was ended")]
fn nothing_pasted_before_end(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let before = &run.events()[..run.mark];
    if let Some(paste) = before.iter().find(|e| matches!(e, Event::Pasted(_))) {
        bail!("pasted before the dictation was ended: {paste:?}; events: {before:?}");
    }
    run.wait_since_mark(|e| matches!(e, Event::Pasted(_)))
}

#[then("the listening-on sound played when the service came back")]
fn listening_on_after_return(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).events();
    let off = events
        .iter()
        .position(|e| *e == Event::Sound("listening off"));
    let on_after = off.and_then(|off| {
        events[off..]
            .iter()
            .position(|e| *e == Event::Sound("listening on"))
    });
    match on_after {
        Some(_) => Ok(()),
        None => bail!("no listening-on sound after the listening-off sound; events: {events:?}"),
    }
}

#[then("a dictation starts")]
fn dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[then("listening is on")]
#[then("listening stays on")]
fn listening_on(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    std::thread::sleep(SETTLE);
    if !run.listening() {
        bail!("listening is off; events: {:?}", run.events());
    }
    Ok(())
}

#[then(expr = "saying {string} presses Return")]
fn saying_presses_return(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone(&phrase)?;
    run.wait_since_mark(|e| *e == Event::ReturnPressed)
}

#[then("the overlay is hidden")]
fn overlay_hidden(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::Overlay("hidden"))
}

#[then("the overlay shows the recording")]
fn overlay_recording(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::Overlay("recording"))
}

#[then("the overlay shows processing until the sentence is pasted, then hides")]
fn overlay_processing_then_hidden(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Overlay("hidden"))?;
    let since = run.since_mark();
    let at = |wanted: &Event| since.iter().position(|e| e == wanted);
    let processing = at(&Event::Overlay("processing"));
    let pasted = since.iter().position(|e| matches!(e, Event::Pasted(_)));
    let hidden = at(&Event::Overlay("hidden"));
    match (processing, pasted, hidden) {
        (Some(p), Some(t), Some(h)) if p < t && t < h => Ok(()),
        _ => bail!("expected processing, paste, hidden in order; events: {since:?}"),
    }
}

#[then(expr = "the {word} sound plays")]
fn sound_plays(world: &mut IntegrationWorld, sound: String) -> Result<()> {
    let name = match sound.as_str() {
        "listening-on" => "listening on",
        "listening-off" => "listening off",
        "start" => "start",
        "stop" => "stop",
        "cancel" => "cancel",
        "command" => "command",
        other => bail!("unknown sound {other}"),
    };
    run(world).wait_since_mark(|e| *e == Event::Sound(name))
}

#[then("no sound plays")]
fn no_sound(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Sound(_)), "a sound played")
}

#[then("the sentence is pasted into the focused app")]
fn sentence_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if !pasted.contains(SENTENCE) {
        bail!("pasted {pasted:?}, expected the sentence {SENTENCE:?}");
    }
    Ok(())
}

#[then("the pasted text does not contain the stop phrase")]
fn no_stop_phrase(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if pasted.contains(STOP_PHRASE) {
        bail!("the stop phrase reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then(expr = "the pasted text does not contain {string}")]
fn pasted_without(world: &mut IntegrationWorld, word: String) -> Result<()> {
    let pasted = run(world).pasted()?;
    let has_word = pasted
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .any(|w| w == word);
    if has_word {
        bail!("{word:?} reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then("the pasted text does not contain the words after it")]
fn pasted_without_more_words(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if pasted.contains(MORE_WORDS) {
        bail!("the words after the command reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then("a new dictation starts")]
fn new_dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::RecordingStarted)
}

/// Said after a restart: the only text of the dictation that follows.
const OTHER_SENTENCE: &str = "Lunch is at noon.";
/// Words of the commands that start a dictation over: none may be pasted.
const RESTART_WORDS: [&str; 3] = ["restart", "start", "dictation"];

#[given(expr = "the user said a sentence, {string}, and another sentence")]
fn restarted_with_another_sentence(world: &mut IntegrationWorld, restart: String) -> Result<()> {
    dictation_runs(world)?;
    let run = run(world);
    run.speak(SENTENCE, Instant::now());
    run.say_alone(short_form(&restart).context("no command")?.phrase())?;
    let deadline = Instant::now() + REACTION;
    let started = |run: &VoiceRun| {
        run.events()
            .iter()
            .filter(|e| **e == Event::RecordingStarted)
            .count()
    };
    while started(run) < 2 {
        if Instant::now() >= deadline {
            bail!("no new dictation after the restart: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    run.speak(OTHER_SENTENCE, Instant::now());
    Ok(())
}

#[then("only the other sentence is pasted")]
#[then("only the other sentence is pasted, its first words included")]
fn only_other_sentence(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    let command_word = pasted
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .any(|w| RESTART_WORDS.contains(&w));
    if command_word || pasted.contains(SENTENCE) || !pasted.contains(OTHER_SENTENCE) {
        bail!("pasted {pasted:?}, expected only {OTHER_SENTENCE:?}");
    }
    Ok(())
}

#[then("the cancel sound plays, then the start sound")]
fn cancel_then_start_sound(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("start"))?;
    let since = run.since_mark();
    let at = |name| since.iter().position(|e| *e == Event::Sound(name));
    match (at("cancel"), at("start")) {
        (Some(cancel), Some(start)) if cancel < start => Ok(()),
        _ => bail!("expected the cancel sound, then the start sound: {since:?}"),
    }
}

#[then("a new dictation starts after it")]
fn new_dictation_after(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted_at = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let deadline = Instant::now() + REACTION;
    while !run.events()[pasted_at..].contains(&Event::RecordingStarted) {
        if Instant::now() >= deadline {
            bail!("no dictation started after the paste: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[then("the sentence is pasted, then select all and delete are pressed, then the Return key is pressed")]
fn pasted_cleared_sent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let cleared = run.wait_for(|e| *e == Event::AllTextCleared)?;
    let sent = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if !(pasted < cleared && cleared < sent) {
        bail!("expected paste, clear, Return in order: {:?}", run.events());
    }
    Ok(())
}

#[then("no text is pasted")]
fn no_text_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Pasted(_)), "text was pasted")
}

#[then("the Return key is pressed in the focused app")]
fn return_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ReturnPressed)?;
    Ok(())
}

#[then("select all and delete are pressed in the focused app")]
fn all_text_cleared(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::AllTextCleared)?;
    Ok(())
}

#[then("the dictation keeps running")]
fn dictation_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let ended = |e: &Event| {
        matches!(
            e,
            Event::Pasted(_) | Event::RecordingCancelled | Event::Sound("stop")
        )
    };
    run(world).never_since_mark(ended, "the dictation ended")
}

#[then("no key is pressed")]
fn no_key_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(is_key_press, "a key was pressed")
}

#[then("the text is pasted first")]
fn text_pasted_first(world: &mut IntegrationWorld) -> Result<()> {
    run(world).pasted()?;
    Ok(())
}

#[then("the Return key is pressed after it")]
fn return_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted_at = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let return_at = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if return_at < pasted_at {
        bail!("Return was pressed before the paste: {:?}", run.events());
    }
    Ok(())
}

#[then("an error naming the command spotting service is logged")]
fn error_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let deadline = Instant::now() + REACTION;
    loop {
        let errors = error_log().0.lock().unwrap().clone();
        if errors
            .iter()
            .any(|e| e.contains("command spotting service"))
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("no error naming the command spotting service; errors: {errors:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
