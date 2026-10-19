//! Drives the real pipeline on its own thread with stand-ins that fail as a scenario sets
//! them, and watches in one ordered log what the overlay shows and what is pasted.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, KeyPresser, PipelineInput, SoundPlayer, TextPaster,
    TextRefiner, Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AccessibilityMissing, AudioBuffer16k, CleanedText, FailingPart, FailureNotice, PttSignal,
    RawTranscript, RecordingMode, SpottingEvent, VoiceCommand,
};

use crate::IntegrationWorld;

const RAW_TRANSCRIPT: &str = "meeting moved to friday";
const REFINED_TEXT: &str = "Meeting moved to Friday.";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How soon after the stop the failure notice must show.
const NOTICE_DEADLINE: Duration = Duration::from_secs(2);
/// Long enough for every overlay signal after the end of a dictation to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// The chunk threshold that makes a dictation long: it is transcribed in parts.
const LONG_DICTATION_THRESHOLD_SECS: u32 = 5;
const LOCAL_SERVICES_REMEDY: &str = "start the local services";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    RecordingStarted,
    Pasted(String),
    Overlay(Shown),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Recording,
    Stage,
    PastedSign,
    Hidden,
    Notice(FailureNotice),
}

type Events = Arc<Mutex<Vec<Event>>>;

fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Recognition {
    #[default]
    Answers,
    Fails,
    FailsOnFirstPart,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Refinement {
    #[default]
    Answers,
    Fails,
    Off,
}

/// How the stand-ins behave in a scenario.
#[derive(Debug, Default, Clone, Copy)]
struct Setup {
    recognition: Recognition,
    refinement: Refinement,
    paste_fails: bool,
    microphone_fails: bool,
    long_dictation: bool,
    voice_control: bool,
    keys_refused: bool,
    spotting_down: bool,
}

/// What a failure-notices scenario shares with the pipeline thread.
#[derive(Default)]
pub struct NoticesRun {
    events: Events,
    setup: Setup,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log and moment where the latest recording was ended.
    stop: Option<(usize, Instant)>,
    /// Moment the failure notice was first seen after the stop.
    notice_seen: Option<Instant>,
}

impl std::fmt::Debug for NoticesRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoticesRun")
            .field("events", &self.events())
            .finish()
    }
}

impl NoticesRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn after_stop(&self) -> Vec<Event> {
        let from = self.stop.map_or(0, |(at, _)| at);
        self.events()[from..].to_vec()
    }

    /// Starts the pipeline the way the app does, once.
    fn start(&mut self) {
        if self.inputs.is_none() {
            self.inputs = Some(start_pipeline(&self.events, self.setup));
        }
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        self.start();
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)
            .map_err(|_| anyhow!("the pipeline stopped; events: {:?}", self.events()))
    }

    fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts a recording with the hotkey and waits until it runs.
    fn start_hotkey_dictation(&mut self) -> Result<()> {
        let from = self.events().len();
        self.press_hotkey()?;
        self.wait_for_after(from, &Event::Overlay(Shown::Recording))?;
        if self.setup.long_dictation {
            // the first part is transcribed while the user still speaks
            std::thread::sleep(SETTLE);
        }
        Ok(())
    }

    fn start_voice_dictation(&mut self) -> Result<()> {
        let from = self.events().len();
        self.say(VoiceCommand::StartDictation)?;
        self.wait_for_after(from, &Event::Overlay(Shown::Recording))
    }

    fn say(&mut self, command: VoiceCommand) -> Result<()> {
        let segment_start = Instant::now();
        let start = segment_start + Duration::from_millis(100);
        let end = start + Duration::from_millis(800);
        let events = [
            SpottingEvent::SpeechStarted(segment_start),
            SpottingEvent::PhraseSpotted {
                phrase: command.phrase().to_owned(),
                start,
                end,
                timing: None,
            },
            SpottingEvent::SpeechEnded(end + Duration::from_millis(100)),
        ];
        for event in events {
            self.send(PipelineInput::Spotting(event))?;
        }
        Ok(())
    }

    fn mark_stop(&mut self) {
        self.stop = Some((self.events().len(), Instant::now()));
        self.notice_seen = None;
    }

    fn wait_for_after(&self, from: usize, wanted: &Event) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.events()[from..].contains(wanted) {
            if Instant::now() >= deadline {
                bail!("{wanted:?} not seen; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Waits until the dictation is over: the overlay hides, or shows a notice or the Pasted
    /// sign that takes itself down, after the stop.
    fn wait_until_over(&mut self) -> Result<Vec<Event>> {
        let deadline = Instant::now() + REACTION;
        loop {
            let shown = overlay(&self.after_stop());
            if let Some(Shown::Notice(_)) = shown.last() {
                self.notice_seen.get_or_insert_with(Instant::now);
                break;
            }
            if matches!(shown.last(), Some(Shown::Hidden | Shown::PastedSign)) {
                break;
            }
            if Instant::now() >= deadline {
                bail!("the dictation never ended; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        Ok(self.after_stop())
    }

    /// Waits until the overlay settles after an action that does not end a dictation.
    fn settled(&self) -> Vec<Event> {
        std::thread::sleep(SETTLE);
        self.after_stop()
    }
}

fn start_pipeline(events: &Events, setup: Setup) -> Sender<PipelineInput> {
    let (tx, rx) = mpsc::channel();
    let refiner: Box<dyn TextRefiner> = match setup.refinement {
        Refinement::Off => Box::new(PassthroughTextRefiner),
        Refinement::Answers => Box::new(ScriptedRefiner { fails: false }),
        Refinement::Fails => Box::new(ScriptedRefiner { fails: true }),
    };
    let threshold = if setup.long_dictation {
        LONG_DICTATION_THRESHOLD_SECS
    } else {
        0
    };
    let mut pipeline = Pipeline::new(
        Box::new(PartsRecorder::new(Arc::clone(events), setup)),
        Box::new(ScriptedTranscriber::new(setup.recognition)),
        refiner,
        Box::new(ScriptedPaster {
            events: Arc::clone(events),
            fails: setup.paste_fails,
        }),
        Box::new(Silent),
        Box::new(NotingOverlay(Arc::clone(events))),
        PipelineConfig {
            min_press_ms: 0,
            recording_mode: RecordingMode::Toggle,
            chunking: ChunkingCfg {
                chunk_threshold_secs: threshold,
                ..ChunkingCfg::default()
            },
        },
    );
    if setup.refinement == Refinement::Off {
        pipeline = pipeline.without_text_refinement_stage();
    }
    if setup.voice_control {
        pipeline = pipeline.with_voice_control(
            Box::new(QuietSpotter {
                down: setup.spotting_down,
            }),
            Box::new(ScriptedKeys {
                refused: setup.keys_refused,
            }),
        );
    }
    std::thread::spawn(move || pipeline.run(rx));
    tx
}

fn run(world: &mut IntegrationWorld) -> &mut NoticesRun {
    world.notices.get_or_insert_with(NoticesRun::default)
}

/// The overlay signals in `events`, in order.
fn overlay(events: &[Event]) -> Vec<Shown> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(*shown),
            _ => None,
        })
        .collect()
}

fn notices(events: &[Event]) -> Vec<FailureNotice> {
    overlay(events)
        .into_iter()
        .filter_map(|shown| match shown {
            Shown::Notice(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

fn pasted(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Pasted(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The one notice the dictation that just ended showed.
fn shown_notice(run: &mut NoticesRun) -> Result<FailureNotice> {
    let events = run.wait_until_over()?;
    match notices(&events)[..] {
        [notice] => Ok(notice),
        ref shown => bail!("expected one failure notice, got {shown:?}; events: {events:?}"),
    }
}

fn expect_notice(world: &mut IntegrationWorld, expected: FailureNotice) -> Result<()> {
    let notice = shown_notice(run(world))?;
    if notice != expected {
        bail!("expected the notice {expected:?}, got {notice:?}");
    }
    Ok(())
}

/// Shows a notice that speech recognition failed: one hotkey dictation it does not answer.
fn show_recognition_notice(run: &mut NoticesRun) -> Result<()> {
    run.setup.recognition = Recognition::Fails;
    run.start_hotkey_dictation()?;
    run.mark_stop();
    run.press_hotkey()?;
    shown_notice(run)?;
    Ok(())
}

#[given("speech recognition is not running")]
#[given("speech recognition does not answer")]
fn recognition_fails(world: &mut IntegrationWorld) {
    run(world).setup.recognition = Recognition::Fails;
}

#[given("speech recognition fails on one part of a long dictation and answers the others")]
fn recognition_fails_on_one_part(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.recognition = Recognition::FailsOnFirstPart;
    run.setup.long_dictation = true;
}

#[given("text refinement is on and its service is down")]
fn refinement_down(world: &mut IntegrationWorld) {
    run(world).setup.refinement = Refinement::Fails;
}

#[given("text refinement is turned off")]
fn refinement_off(world: &mut IntegrationWorld) {
    run(world).setup.refinement = Refinement::Off;
}

#[given("pasting fails")]
fn pasting_fails(world: &mut IntegrationWorld) {
    run(world).setup.paste_fails = true;
}

#[given("the microphone cannot be opened")]
fn microphone_fails(world: &mut IntegrationWorld) {
    run(world).setup.microphone_fails = true;
}

#[given("the user is recording a hotkey dictation")]
#[given("the user is recording a long hotkey dictation")]
fn records_hotkey_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_hotkey_dictation()
}

#[given("voice control is turned on and a dictation started by voice runs")]
fn records_voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.setup.voice_control = true;
    run.start_voice_dictation()
}

#[given("voice control is turned on and pressing keys in the focused app is refused")]
fn keys_refused(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.voice_control = true;
    run.setup.keys_refused = true;
}

#[given("voice control is turned on and the command spotting service is not running")]
fn spotting_down(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.voice_control = true;
    run.setup.spotting_down = true;
}

#[given("a failure notice is shown")]
#[given("a failure notice that speech recognition failed is shown")]
fn notice_shown(world: &mut IntegrationWorld) -> Result<()> {
    show_recognition_notice(run(world))
}

#[when("the user stops the hotkey dictation")]
fn stops_hotkey_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.press_hotkey()
}

#[when(expr = "the user ends the voice dictation by saying {string}")]
fn ends_voice_dictation(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .filter(|command| *command == VoiceCommand::StopDictation)
        .with_context(|| {
            format!("only \"stop dictation\" ends a dictation here, not {phrase:?}")
        })?;
    let run = run(world);
    run.mark_stop();
    run.say(command)
}

#[when("the user presses the hotkey to record")]
fn presses_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.press_hotkey()
}

#[when(expr = "the user gives the voice command {string}")]
fn says(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .with_context(|| format!("{phrase:?} is not a voice command"))?;
    let run = run(world);
    run.mark_stop();
    run.say(command)
}

#[when("the app is launched")]
fn app_starts(world: &mut IntegrationWorld) {
    let run = run(world);
    run.mark_stop();
    run.start();
}

#[when("the next dictation fails the same way")]
fn next_dictation_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    // the log of the previous dictation stays in view: no hide may come between the notices
    let previous = run.stop;
    run.start_hotkey_dictation()?;
    run.press_hotkey()?;
    run.stop = previous;
    Ok(())
}

#[then("within 2 seconds of the stop the overlay shows a failure notice naming speech recognition")]
fn notice_names_recognition_in_time(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let notice = shown_notice(run)?;
    if notice.part() != Some(FailingPart::SpeechRecognition) {
        bail!("the notice does not name speech recognition: {notice:?}");
    }
    let (_, stopped) = run.stop.context("the dictation was never stopped")?;
    let seen = run.notice_seen.context("the notice was never seen")?;
    if seen.duration_since(stopped) > NOTICE_DEADLINE {
        bail!("the notice took {:?} after the stop", seen - stopped);
    }
    Ok(())
}

#[then("the notice says to start the local services")]
fn notice_names_remedy(world: &mut IntegrationWorld) -> Result<()> {
    let message = shown_notice(run(world))?.message();
    if !message.contains(LOCAL_SERVICES_REMEDY) {
        bail!("the notice does not say to start the local services: {message:?}");
    }
    Ok(())
}

#[then("the dictation's text is not pasted")]
fn nothing_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if !pasted(&events).is_empty() {
        bail!("text was pasted: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the dictation was lost because speech recognition failed")]
fn dictation_lost(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::DictationLost(FailingPart::SpeechRecognition),
    )
}

#[then("the text of the other parts is pasted")]
fn other_parts_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if pasted(&events) != [REFINED_TEXT] {
        bail!("expected {REFINED_TEXT:?} pasted; events: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that part of the dictation was lost")]
fn part_lost(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::PartOfDictationLost(FailingPart::SpeechRecognition),
    )
}

#[then("the raw transcript is pasted")]
fn raw_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if pasted(&events) != [RAW_TRANSCRIPT] {
        bail!("expected the raw transcript pasted; events: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the text was pasted without refinement")]
fn pasted_unrefined(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::PastedUnrefined(FailingPart::TextRefinement),
    )
}

#[then("the overlay shows no failure notice")]
fn no_notice(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if !notices(&events).is_empty() {
        bail!("a failure notice was shown: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the text was not pasted")]
fn not_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let notice = shown_notice(run(world))?;
    if !matches!(notice, FailureNotice::NotPasted(_)) {
        bail!("expected a notice that the text was not pasted, got {notice:?}");
    }
    Ok(())
}

#[then("the recording does not start")]
fn recording_not_started(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if events.contains(&Event::RecordingStarted) || overlay(&events).contains(&Shown::Recording) {
        bail!("a recording started: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the microphone is unavailable")]
fn microphone_unavailable(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if notices(&events) != [FailureNotice::RecordingNotStarted] {
        bail!("expected the notice that the recording did not start; events: {events:?}");
    }
    Ok(())
}

#[then(
    expr = "the overlay shows a failure notice that {string} failed for lack of the Accessibility permission"
)]
fn command_failed(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .with_context(|| format!("{phrase:?} is not a voice command"))?;
    let expected = FailureNotice::CommandFailed(command, Some(FailingPart::Accessibility));
    let events = run(world).settled();
    if notices(&events) != [expected] {
        bail!("expected the notice {expected:?}; events: {events:?}");
    }
    Ok(())
}

#[then("the next voice command is still carried out")]
fn next_command_carried_out(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let from = run.events().len();
    run.say(VoiceCommand::StartDictation)?;
    run.wait_for_after(from, &Event::RecordingStarted)
}

#[then("the overlay shows a failure notice that voice commands are unavailable because command spotting is not running")]
fn voice_commands_unavailable(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if notices(&events) != [FailureNotice::VoiceCommandsUnavailable] {
        bail!("expected the notice that voice commands are unavailable; events: {events:?}");
    }
    Ok(())
}

#[then("the recording starts and the overlay shows the recording instead of the notice")]
fn recording_replaces_notice(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if !events.contains(&Event::RecordingStarted) {
        bail!("no recording started: {events:?}");
    }
    if overlay(&events).last() != Some(&Shown::Recording) {
        bail!("the overlay does not show the recording: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the same failure notice again without hiding in between")]
fn same_notice_again(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let events = run.wait_until_over()?;
    let shown = overlay(&events);
    let at: Vec<usize> = (0..shown.len())
        .filter(|i| matches!(shown[*i], Shown::Notice(_)))
        .collect();
    let [first, second] = at[..] else {
        bail!("expected two failure notices; overlay: {shown:?}");
    };
    if shown[first] != shown[second] {
        bail!("the second notice differs: {shown:?}");
    }
    if shown[first..second].contains(&Shown::Hidden) {
        bail!("the overlay hid between the notices: {shown:?}");
    }
    Ok(())
}

/// Hands over one second of audio at the stop; for a long dictation it sends one part while
/// the user speaks and the last one when the recording is flushed.
struct PartsRecorder {
    events: Events,
    setup: Setup,
    parts: Option<mpsc::Sender<LiveChunk>>,
}

impl PartsRecorder {
    fn new(events: Events, setup: Setup) -> Self {
        Self {
            events,
            setup,
            parts: None,
        }
    }

    fn one_second() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize])
    }

    fn send_part(&self) {
        if let Some(parts) = &self.parts {
            let _ = parts.send(LiveChunk {
                audio: Self::one_second(),
                enqueued_at: Instant::now(),
            });
        }
    }
}

impl AudioRecorder for PartsRecorder {
    fn start(&mut self) -> Result<()> {
        if self.setup.microphone_fails {
            bail!("the microphone cannot be opened");
        }
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(Self::one_second())
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.setup.long_dictation {
            return Ok(None);
        }
        let (tx, rx) = mpsc::channel();
        self.parts = Some(tx);
        self.send_part();
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        self.send_part();
        // the transcription of the parts ends with the last one
        self.parts = None;
    }
}

struct ScriptedTranscriber {
    recognition: Recognition,
    calls: usize,
}

impl ScriptedTranscriber {
    fn new(recognition: Recognition) -> Self {
        Self {
            recognition,
            calls: 0,
        }
    }
}

impl Transcriber for ScriptedTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        self.calls += 1;
        let fails = match self.recognition {
            Recognition::Answers => false,
            Recognition::Fails => true,
            Recognition::FailsOnFirstPart => self.calls == 1,
        };
        if fails {
            bail!("connection refused");
        }
        Ok(Some(RawTranscript::new(RAW_TRANSCRIPT.to_owned())?))
    }
}

struct ScriptedRefiner {
    fails: bool,
}

impl TextRefiner for ScriptedRefiner {
    fn refine(&mut self, _raw: &RawTranscript) -> Result<CleanedText> {
        if self.fails {
            bail!("connection refused");
        }
        Ok(CleanedText::new(REFINED_TEXT.to_owned())?)
    }
}

struct ScriptedPaster {
    events: Events,
    fails: bool,
}

impl TextPaster for ScriptedPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        if self.fails {
            bail!("the paste was refused");
        }
        record(&self.events, Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

struct NotingOverlay(Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        record(&self.0, Event::Overlay(Shown::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.0, Event::Overlay(Shown::Stage));
    }

    fn show_refining_text(&self, _raw: &RawTranscript) {
        record(&self.0, Event::Overlay(Shown::Stage));
    }

    fn show_pasted(&self, _text: &CleanedText) {
        record(&self.0, Event::Overlay(Shown::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.0, Event::Overlay(Shown::Stage));
    }

    fn hide(&self) {
        record(&self.0, Event::Overlay(Shown::Hidden));
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        record(&self.0, Event::Overlay(Shown::Notice(*notice)));
    }
}

/// The command spotting service as the pipeline sees it; the scenario feeds the phrases it
/// reports straight into the pipeline's input channel.
struct QuietSpotter {
    /// The service is not running: listening cannot start.
    down: bool,
}

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        if self.down {
            bail!("connection refused");
        }
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
}

/// Presses keys, or is refused them the way macOS refuses an app without the
/// Accessibility permission.
struct ScriptedKeys {
    refused: bool,
}

impl ScriptedKeys {
    fn press(&self) -> Result<()> {
        if self.refused {
            return Err(AccessibilityMissing.into());
        }
        Ok(())
    }
}

impl KeyPresser for ScriptedKeys {
    fn press_enter(&mut self) -> Result<()> {
        self.press()
    }

    fn clear_all_text(&mut self) -> Result<()> {
        self.press()
    }

    fn paste(&mut self) -> Result<()> {
        self.press()
    }
}

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}
