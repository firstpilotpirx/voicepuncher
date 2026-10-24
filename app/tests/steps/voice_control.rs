//! Drives the real pipeline on its own thread with voice control: a scenario speaks by
//! sending the speech and phrase events the command spotting service would report, and
//! watches the ordered log of what the stand-ins in `doubles` were asked to do.

mod doubles;
mod failed_confirmation;
mod speech_after_start;
mod start_timing;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{CommandPlacement, PttSignal, SpottingEvent, VoiceCommand};

use doubles::{
    Cut, Event, Events, NotingRecorder, Phrases, PipelineLog, RecordingKeyPresser,
    RecordingOverlay, RecordingPaster, RecordingSounds, ScriptedSpotter, Speech, SpeechBeforeCut,
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

/// The overlay event of the Pasted sign.
pub(crate) const PASTED_SIGN: &str = "pasted sign";
/// The overlay events of the command signs.
const COMMAND_SIGNS: [&str; 3] = ["enter sign", "clear sign", "restart sign"];

static PIPELINE_LOG: OnceLock<&'static PipelineLog> = OnceLock::new();

pub(crate) fn pipeline_log() -> &'static PipelineLog {
    PIPELINE_LOG.get_or_init(|| {
        let log: &'static PipelineLog = Box::leak(Box::default());
        if log::set_logger(log).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
        log
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
    /// The thread the pipeline runs on: what it logs is the scenario's.
    pipeline_thread: Option<ThreadId>,
    /// How much microphone audio never reached the command spotting service.
    audio_lost: Duration,
    /// When the words of the scenario's command ended.
    command_end: Option<Instant>,
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

    fn start_pipeline(&mut self) -> Sender<PipelineInput> {
        pipeline_log();
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(NotingRecorder {
                    events: Arc::clone(&self.events),
                    speech: Arc::clone(&self.speech),
                    cut: Arc::clone(&self.cut),
                    said_before: Vec::new(),
                }),
                transcriber: Box::new(SpeechBeforeCut {
                    speech: Arc::clone(&self.speech),
                    cut: Arc::clone(&self.cut),
                }),
                paster: Box::new(RecordingPaster(Arc::clone(&self.events))),
                sound_player: Box::new(RecordingSounds(Arc::clone(&self.events))),
                visual_indicator: Box::new(RecordingOverlay(Arc::clone(&self.events))),
            },
            PipelineConfig::default(),
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
        let pipeline_thread = std::thread::spawn(move || pipeline.run(rx));
        self.pipeline_thread = Some(pipeline_thread.thread().id());
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
            timing: None,
        }))
    }

    fn speak(&self, words: &str, at: Instant) {
        self.speech.lock().unwrap().push((at, words.to_owned()));
    }

    #[expect(clippy::too_many_arguments, reason = "size baseline")]
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
                timing: None,
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

pub(crate) fn run(world: &mut IntegrationWorld) -> &mut VoiceRun {
    world.voice.get_or_insert_with(VoiceRun::default)
}

fn is_key_press(e: &Event) -> bool {
    matches!(
        e,
        Event::ReturnPressed | Event::AllTextCleared | Event::PasteKeyPressed
    )
}

#[given("voice control is not turned on")]
fn voice_control_off(world: &mut IntegrationWorld) {
    run(world).voice_control = false;
}

pub(crate) fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("no dictation runs")]
fn no_dictation_given(_world: &mut IntegrationWorld) {}

pub(crate) fn listening_on_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    run.wait_for(|e| *e == Event::Sound("listening on"))?;
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

pub(crate) fn app_starts(world: &mut IntegrationWorld) {
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
pub(crate) fn records_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.dictate_with_hotkey()?;
    run.press_hotkey()?;
    run.pasted().map(drop)
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
        "paste" => Some(VoiceCommand::PasteText),
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
    std::thread::sleep(SETTLE);
    Ok(())
}

#[given("the command spotting service went away and is back")]
pub(crate) fn service_went_away_and_is_back(world: &mut IntegrationWorld) -> Result<()> {
    service_went_away(world)?;
    let run = run(world);
    run.send(PipelineInput::SpottingBack)?;
    std::thread::sleep(SETTLE);
    Ok(())
}

#[when(expr = "the service is back and reports {string}, said while it was away")]
fn back_with_command_said_away(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    let away_since = run.away_since.context("the service did not go away")?;
    run.start_when();
    run.send(PipelineInput::SpottingBack)?;
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

pub(crate) fn no_listening_sound_after_start(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let events = run.events();
    let listening_sounds = events
        .iter()
        .filter(|e| matches!(e, Event::Sound("listening on" | "listening off")))
        .count();
    if listening_sounds != 1 {
        bail!("expected only the listening-on sound at start; events: {events:?}");
    }
    Ok(())
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
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    run.wait_since_mark(|e| *e == Event::Overlay(PASTED_SIGN))?;
    let since = run.since_mark();
    let at = |wanted: &Event| since.iter().position(|e| e == wanted);
    // processing starts with its first stage, speech recognition
    let processing = at(&Event::Overlay("recognizing speech"));
    let pasted = since.iter().position(|e| matches!(e, Event::Pasted(_)));
    let signed = at(&Event::Overlay(PASTED_SIGN));
    match (processing, pasted, signed) {
        (Some(p), Some(t), Some(s)) if p < t && t < s => Ok(()),
        _ => bail!("expected processing, paste, the Pasted sign in order; events: {since:?}"),
    }
}

#[then(expr = "the {word} sound plays")]
fn sound_plays(world: &mut IntegrationWorld, sound: String) -> Result<()> {
    let name = sound_of(&sound)?;
    run(world).wait_since_mark(|e| *e == Event::Sound(name))
}

/// The event name of a sound as a scenario names it.
fn sound_of(sound: &str) -> Result<&'static str> {
    Ok(match sound {
        "listening-on" => "listening on",
        "listening-off" => "listening off",
        "start" => "start",
        "stop" => "stop",
        "cancel" => "cancel",
        "enter" => "enter",
        "clear" => "clear",
        "paste" => "paste",
        "restart" => "restart",
        other => bail!("unknown sound {other}"),
    })
}

#[then("no sound plays")]
fn no_sound(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Sound(_)), "a sound played")
}

pub(crate) fn sentence_pasted(world: &mut IntegrationWorld) -> Result<()> {
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
pub(crate) fn only_other_sentence(world: &mut IntegrationWorld) -> Result<()> {
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

#[then("the restart sound plays, and neither the cancel sound nor the start sound")]
fn restart_sound_alone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("restart"))?;
    run.never_since_mark(
        |e| matches!(e, Event::Sound("cancel" | "start")),
        "a cancel or start sound played",
    )
}

/// The overlay events before the step's action.
pub(crate) fn overlay_before_mark(run: &VoiceRun) -> Vec<&'static str> {
    run.events()[..run.mark]
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(*shown),
            _ => None,
        })
        .collect()
}

/// The overlay events since the step's action.
pub(crate) fn overlay_since_mark(run: &VoiceRun) -> Vec<&'static str> {
    run.since_mark()
        .into_iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(shown),
            _ => None,
        })
        .collect()
}

/// Waits for `sign` and checks nothing replaced it: the overlay takes it down on its own after
/// about a second, back to what it showed before.
pub(crate) fn ends_on_sign(run: &VoiceRun, sign: &'static str) -> Result<Vec<&'static str>> {
    run.wait_since_mark(|e| *e == Event::Overlay(sign))?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    if shown.last() != Some(&sign) {
        bail!("expected the overlay to end on the {sign}: {shown:?}");
    }
    Ok(shown)
}

#[then(expr = "the overlay shows the {word} sign for about a second, then hides")]
fn sign_then_hides(world: &mut IntegrationWorld, command: String) -> Result<()> {
    ends_on_sign(run(world), sign_of(&command)?)?;
    Ok(())
}

#[then("the overlay shows the restart sign, then the recording")]
fn restart_sign_then_recording(world: &mut IntegrationWorld) -> Result<()> {
    // The new recording is shown first; the sign covers it and gives it back after a second.
    let shown = ends_on_sign(run(world), "restart sign")?;
    if !shown.contains(&"recording") {
        bail!("expected the recording under the restart sign: {shown:?}");
    }
    Ok(())
}

#[then(expr = "the {word} sound plays and the overlay shows the {word} sign after the paste")]
fn sound_and_sign_after_paste(
    world: &mut IntegrationWorld,
    sound: String,
    command: String,
) -> Result<()> {
    let run = run(world);
    let sign = sign_of(&command)?;
    let sound = sound_of(&sound)?;
    let pasted = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let sounded = run.wait_for(|e| *e == Event::Sound(sound))?;
    let signed = run.wait_for(|e| *e == Event::Overlay(sign))?;
    if !(pasted < sounded && pasted < signed) {
        bail!(
            "expected the {sound} sound and {sign} after the paste: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then(expr = "the {word} sound plays and the overlay shows the {word} sign")]
fn sound_and_sign(world: &mut IntegrationWorld, sound: String, command: String) -> Result<()> {
    let run = run(world);
    let (sound, sign) = (sound_of(&sound)?, sign_of(&command)?);
    run.wait_since_mark(|e| *e == Event::Sound(sound))?;
    run.wait_since_mark(|e| *e == Event::Overlay(sign))
}

#[then("the overlay shows the recording as soon as the new dictation starts")]
fn recording_after_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Overlay("recording"))?;
    let shown = overlay_since_mark(run);
    let at = |wanted| shown.iter().position(|s| *s == wanted);
    match (at("enter sign"), at("recording")) {
        (Some(sign), Some(recording)) if sign < recording => Ok(()),
        _ => bail!("expected the enter sign, then the recording: {shown:?}"),
    }
}

#[then("the stop sound, the clear sound and the enter sound play in this order")]
fn stop_clear_enter_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("enter"))?;
    let since = run.since_mark();
    let at = |name| since.iter().position(|e| *e == Event::Sound(name));
    match (at("stop"), at("clear"), at("enter")) {
        (Some(stop), Some(clear), Some(enter)) if stop < clear && clear < enter => Ok(()),
        _ => bail!("expected the stop, clear and enter sounds in order: {since:?}"),
    }
}

#[then("the overlay shows the clear sign, then the enter sign, then hides")]
fn clear_then_enter_sign(world: &mut IntegrationWorld) -> Result<()> {
    let shown = ends_on_sign(run(world), "enter sign")?;
    match shown.iter().position(|s| *s == "clear sign") {
        Some(_) => Ok(()),
        None => bail!("expected the clear sign before the enter sign: {shown:?}"),
    }
}

#[when(expr = "the user says {string}, and later {string}")]
fn says_and_later(world: &mut IntegrationWorld, first: String, second: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone(chain(&first).map_or(first.as_str(), |c| c[0].phrase()))?;
    std::thread::sleep(SETTLE);
    run.say_alone(chain(&second).map_or(second.as_str(), |c| c[0].phrase()))
}

#[then("the two commands play different sounds")]
fn two_different_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let deadline = Instant::now() + REACTION;
    loop {
        let sounds: Vec<_> = run
            .since_mark()
            .into_iter()
            .filter(|e| matches!(e, Event::Sound(_)))
            .collect();
        if let [first, second] = sounds.as_slice() {
            if first == second {
                bail!("both commands played {first:?}");
            }
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("expected two command sounds: {sounds:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then("the start and stop sounds play")]
fn start_and_stop_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("start"))?;
    run.wait_since_mark(|e| *e == Event::Sound("stop"))
}

#[then(
    "the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign"
)]
fn hotkey_overlay_without_sign(world: &mut IntegrationWorld) -> Result<()> {
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    let shown = ends_on_sign(run(world), PASTED_SIGN)?;
    if shown.iter().any(|s| COMMAND_SIGNS.contains(s)) {
        bail!("a command sign was shown: {shown:?}");
    }
    let at = |wanted| shown.iter().position(|s| *s == wanted);
    match (at("recording"), at("recognizing speech"), at(PASTED_SIGN)) {
        (Some(r), Some(p), Some(t)) if r < p && p < t => Ok(()),
        _ => bail!("expected recording, processing, the Pasted sign in order: {shown:?}"),
    }
}

/// The overlay event of a command's sign.
fn sign_of(command: &str) -> Result<&'static str> {
    match command {
        "enter" => Ok("enter sign"),
        "clear" => Ok("clear sign"),
        "paste" => Ok("paste sign"),
        "restart" => Ok("restart sign"),
        other => bail!("no sign for {other}"),
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

pub(crate) fn return_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ReturnPressed)?;
    Ok(())
}

#[given("the clipboard holds a sentence")]
fn clipboard_holds_sentence(_world: &mut IntegrationWorld) {
    // The app never reads the clipboard: what it holds is the focused app's to paste.
}

#[then("Cmd+V is pressed in the focused app")]
fn paste_key_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::PasteKeyPressed)?;
    Ok(())
}

#[then("the clipboard still holds the same sentence")]
fn clipboard_untouched(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|e| *e == Event::PasteKeyPressed)?;
    run.never_since_mark(
        |e| matches!(e, Event::Pasted(_)),
        "the clipboard was written",
    )
}

#[then("Cmd+V is pressed, then the Return key is pressed")]
fn pasted_clipboard_then_sent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted = run.wait_for(|e| *e == Event::PasteKeyPressed)?;
    let sent = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if pasted >= sent {
        bail!("expected Cmd+V, then Return: {:?}", run.events());
    }
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

pub(crate) fn return_after_paste(world: &mut IntegrationWorld) -> Result<()> {
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
        let errors = pipeline_log().errors.lock().unwrap().clone();
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

/// The overlay events of the sign that voice commands do not work.
const SIGN_SHOWN: Event = Event::Overlay("commands unavailable shown");
const SIGN_HIDDEN: Event = Event::Overlay("commands unavailable hidden");

/// Reports the service away for longer than the outage grace period: the adapter's timer
/// itself is proven by its integration test.
fn stay_away(run: &mut VoiceRun) -> Result<()> {
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}

pub(crate) fn overlay_hidden_given(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).events();
    if events.contains(&SIGN_SHOWN) {
        bail!("the overlay already shows a sign: {events:?}");
    }
    Ok(())
}

#[given("the overlay shows the recording sign")]
fn overlay_recording_given(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::Overlay("recording"))?;
    Ok(())
}

pub(crate) fn sign_shown_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    stay_away(run)?;
    run.wait_for(|e| *e == SIGN_SHOWN)?;
    Ok(())
}

#[when("the command spotting service goes away and is back within 5 seconds")]
fn short_outage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingBack)
}

#[when("the command spotting service stays away for longer than 5 seconds, then is back")]
fn long_outage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    stay_away(run)?;
    run.send(PipelineInput::SpottingBack)
}

#[when(
    "the user ends the dictation with the hotkey and its text is pasted, and later the service is back"
)]
fn hotkey_end_then_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()?;
    run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    std::thread::sleep(SETTLE);
    run.send(PipelineInput::SpottingBack)
}

#[then("the overlay never shows that voice commands do not work")]
fn sign_never_shown(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| *e == SIGN_SHOWN, "the sign was shown")
}

#[then("the overlay shows that voice commands do not work while the service is away")]
fn sign_shown_while_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_SHOWN)?;
    let shown = overlay_since_mark(run);
    if shown.first() != Some(&"commands unavailable shown") {
        bail!("expected the sign before the service was back: {shown:?}");
    }
    Ok(())
}

#[then("the overlay shows that voice commands do not work beside the recording sign")]
fn sign_beside_recording(world: &mut IntegrationWorld) -> Result<()> {
    sign_shown_while_away(world)?;
    still_recording(run(world))
}

/// Nothing ended the dictation since the step's action.
fn still_recording(run: &VoiceRun) -> Result<()> {
    if let Some(end) = run
        .since_mark()
        .into_iter()
        .find(|e| matches!(e, Event::Pasted(_) | Event::RecordingCancelled))
    {
        bail!("the dictation ended: {end:?}; events: {:?}", run.events());
    }
    Ok(())
}

/// Waits for the sign to go and checks the overlay changed nothing else since the action.
fn only_sign_came_and_went(run: &VoiceRun) -> Result<()> {
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    if shown != ["commands unavailable shown", "commands unavailable hidden"] {
        bail!("expected only the sign to come and go: {shown:?}");
    }
    Ok(())
}

#[then("once the service is back, the overlay shows the recording sign alone")]
fn recording_alone_once_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    only_sign_came_and_went(run)?;
    still_recording(run)
}

#[then("once the service is back, the overlay hides")]
fn hidden_once_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    let states: Vec<_> = shown
        .iter()
        .filter(|s| !s.starts_with("commands unavailable"))
        .collect();
    let sign_hidden_last = shown.last() == Some(&"commands unavailable hidden");
    let hidden_once = shown
        .iter()
        .filter(|s| **s == "commands unavailable hidden")
        .count()
        == 1;
    // The Pasted sign takes itself down: a pasted dictation ends hidden on it.
    let ends_hidden = states
        .last()
        .is_none_or(|s| **s == "hidden" || **s == PASTED_SIGN);
    if !sign_hidden_last || !hidden_once || !ends_hidden {
        bail!("expected the overlay to end hidden once the service was back: {shown:?}");
    }
    Ok(())
}

#[then("the overlay still shows that voice commands do not work after the paste")]
fn sign_kept_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    let since = run.since_mark();
    let at = |wanted: &Event| since.iter().position(|e| e == wanted);
    let pasted = since.iter().position(|e| matches!(e, Event::Pasted(_)));
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    let overlay_ended = since
        .iter()
        .rposition(|e| *e == Event::Overlay("hidden") || *e == Event::Overlay(PASTED_SIGN));
    match (pasted, overlay_ended, at(&SIGN_HIDDEN)) {
        (Some(p), Some(h), Some(s)) if p < s && h < s => Ok(()),
        _ => bail!("expected the sign to stay until the service was back; events: {since:?}"),
    }
}
