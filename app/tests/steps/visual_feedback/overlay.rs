//! Drives the real macOS overlay on the main thread, where the harness runs its steps, and
//! reads what its first screen shows through the overlay's own inspectors — the calls the
//! main-thread tests of `visualfeedback_macos` make.

use std::sync::atomic::AtomicU32;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use objc2::MainThreadMarker;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSDate, NSRunLoop};

use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;

use crate::IntegrationWorld;

mod width;
use width::pill_size;

/// A notice that lasts long enough to look at, and short enough not to wait.
const SHORT_NOTICE: Duration = Duration::from_millis(600);
/// Long enough for the main queue to draw what was asked.
const DRAWN_SECS: f64 = 0.2;
/// Long enough for the text window to rise in.
const RISEN_SECS: f64 = 0.8;
/// Long enough for column periods of 200 ms to draw speech.
const SPOKEN_SECS: f64 = 0.6;
/// Silence draws dots this tall; speech draws taller columns.
const DOT_HEIGHT: f64 = 2.0;
/// An RMS level louder than the waveform's full scale.
const SPEECH_LEVEL: f32 = 0.5;
/// The most characters the text window shows, its ellipsis included.
const SHOWN_CHARS: usize = 110;
pub const SO_FAR: &str = "we moved the meeting to Friday";

/// The overlay of a scenario, its microphone level and the pill sizes it saw.
#[derive(Default)]
pub struct OverlayRun {
    overlay: Option<MacosVisualIndicator>,
    level: Arc<AtomicU32>,
    sizes: Vec<(f64, f64)>,
}

impl std::fmt::Debug for OverlayRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayRun")
            .field("sizes", &self.sizes)
            .finish_non_exhaustive()
    }
}

impl Drop for OverlayRun {
    fn drop(&mut self) {
        if let Some(overlay) = &self.overlay {
            overlay.hide_commands_unavailable();
            overlay.hide();
            pump(DRAWN_SECS);
        }
    }
}

pub fn pump(secs: f64) {
    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(secs));
}

fn run(world: &mut IntegrationWorld) -> &mut OverlayRun {
    world
        .extra
        .view
        .overlay
        .get_or_insert_with(Default::default)
}

fn overlay(world: &mut IntegrationWorld) -> Result<&MacosVisualIndicator> {
    run(world)
        .overlay
        .as_ref()
        .context("the overlay is not ready")
}

/// Asks the overlay to show something and lets the main queue draw it.
fn show(world: &mut IntegrationWorld, ask: impl FnOnce(&MacosVisualIndicator)) -> Result<()> {
    ask(overlay(world)?);
    pump(DRAWN_SECS);
    Ok(())
}

#[given("the overlay is ready")]
fn ready(world: &mut IntegrationWorld) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the harness runs its steps on the main thread")?;
    let run = run(world);
    let level = Arc::clone(&run.level);
    run.overlay = Some(MacosVisualIndicator::initialize_with_notice_duration(
        mtm,
        level,
        SHORT_NOTICE,
    ));
    Ok(())
}

#[given("the overlay is ready and the microphone hears speech")]
fn ready_with_speech(world: &mut IntegrationWorld) -> Result<()> {
    ready(world)?;
    run(world)
        .level
        .store(SPEECH_LEVEL.to_bits(), Ordering::Relaxed);
    Ok(())
}

#[when("a recording starts on the overlay")]
fn shows_recording(world: &mut IntegrationWorld) -> Result<()> {
    show(world, MacosVisualIndicator::show_recording)
}

#[when("a recording starts on the overlay and the user speaks")]
fn shows_speech(world: &mut IntegrationWorld) -> Result<()> {
    shows_recording(world).map(|()| pump(SPOKEN_SECS))
}

#[when("the recording ends and speech recognition starts")]
#[when("the overlay shows speech recognition")]
fn shows_recognizing(world: &mut IntegrationWorld) -> Result<()> {
    show(world, MacosVisualIndicator::show_recognizing_speech)
}

#[when("the overlay shows the wait for focus")]
fn shows_waiting(world: &mut IntegrationWorld) -> Result<()> {
    show(world, MacosVisualIndicator::show_waiting_for_focus)
}

#[when("the overlay shows the sign of \"press enter\"")]
fn shows_enter(world: &mut IntegrationWorld) -> Result<()> {
    show(world, |o| o.show_command_sign(VoiceCommand::PressEnter))
}

#[when("the overlay shows that a dictation was lost to speech recognition")]
fn shows_lost_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
    show(world, |o| o.show_failure_notice(&notice))
}

#[when("voice commands stop working while no dictation runs")]
fn commands_lost_in_standby(world: &mut IntegrationWorld) -> Result<()> {
    show(world, MacosVisualIndicator::show_commands_unavailable)
}

#[when("voice commands stop working during a recording")]
fn commands_lost_while_recording(world: &mut IntegrationWorld) -> Result<()> {
    shows_recording(world)?;
    commands_lost_in_standby(world)
}

#[when("the text recognized so far arrives during the recording")]
fn text_so_far(world: &mut IntegrationWorld) -> Result<()> {
    shows_recording(world)?;
    overlay(world)?
        .live_transcript_view()
        .show_live_transcript(SO_FAR);
    pump(RISEN_SECS);
    Ok(())
}

#[when("a text longer than the overlay can show arrives during the recording")]
fn long_text_so_far(world: &mut IntegrationWorld) -> Result<()> {
    shows_recording(world)?;
    let size = pill_size(world)?;
    run(world).sizes.push(size);
    let long = [SO_FAR; 8].join(" and then ");
    overlay(world)?
        .live_transcript_view()
        .show_live_transcript(&long);
    pump(RISEN_SECS);
    Ok(())
}

#[when("the overlay shows the recording, speech recognition and the Pasted sign in turn")]
fn shows_every_dictation_state(world: &mut IntegrationWorld) -> Result<()> {
    let steps: [fn(&MacosVisualIndicator); 3] = [
        MacosVisualIndicator::show_recording,
        MacosVisualIndicator::show_recognizing_speech,
        MacosVisualIndicator::show_pasted,
    ];
    for step in steps {
        show(world, step)?;
        let size = pill_size(world)?;
        run(world).sizes.push(size);
    }
    Ok(())
}

#[then(expr = "the pill shows the icon {string} and the label {string}")]
fn shows_sign(world: &mut IntegrationWorld, symbol: String, label: String) -> Result<()> {
    let overlay = overlay(world)?;
    let shown = (overlay.first_icon_symbol(), overlay.first_label_value());
    ensure!(
        shown == (Some(symbol), Some(label)),
        "the pill shows {shown:?}"
    );
    Ok(())
}

#[then("the pill shows the record sign and a dotted waveform without a label")]
fn record_sign_without_label(world: &mut IntegrationWorld) -> Result<()> {
    let overlay = overlay(world)?;
    ensure!(overlay.first_shows_recording(), "no record sign");
    ensure!(
        overlay.first_label_value().is_none(),
        "the recording shows a label"
    );
    let tallest = overlay.first_tallest_waveform_column();
    ensure!(
        (tallest - DOT_HEIGHT).abs() < f64::EPSILON,
        "a column of {tallest} points"
    );
    Ok(())
}

#[then("the waveform holds a column taller than a dot")]
fn speech_column(world: &mut IntegrationWorld) -> Result<()> {
    let tallest = overlay(world)?.first_tallest_waveform_column();
    ensure!(
        tallest > DOT_HEIGHT,
        "the tallest column is {tallest} points"
    );
    Ok(())
}

#[then("the record dot pulses unless Reduce Motion is on")]
fn pulses_unless_reduced(world: &mut IntegrationWorld) -> Result<()> {
    let moves = overlay(world)?.first_shows_motion();
    let reduced = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
    ensure!(
        moves != reduced,
        "motion {moves} with Reduce Motion {reduced}"
    );
    Ok(())
}

#[then("no text window shows")]
fn no_text_window(world: &mut IntegrationWorld) -> Result<()> {
    let shown = overlay(world)?.first_text_window_value();
    ensure!(shown.is_none(), "a text window shows {shown:?}");
    Ok(())
}

#[then("the text window shows that text whole below the pill")]
fn text_below_pill(world: &mut IntegrationWorld) -> Result<()> {
    let overlay = overlay(world)?;
    let shown = overlay.first_text_window_value();
    ensure!(
        shown.as_deref() == Some(SO_FAR),
        "the text window shows {shown:?}"
    );
    let pill = overlay.first_pill_glass_frame().context("no pill shows")?;
    let text = overlay
        .first_text_window_glass_frame()
        .context("no text window shows")?;
    let top = text.origin.y + text.size.height;
    ensure!(
        top < pill.origin.y,
        "the text window (top {top}) reaches into the pill"
    );
    Ok(())
}

#[then("the text window shows the latest words after an ellipsis and the pill keeps its size")]
fn latest_words(world: &mut IntegrationWorld) -> Result<()> {
    let shown = overlay(world)?
        .first_text_window_value()
        .context("no text window shows")?;
    ensure!(shown.starts_with('…') && shown.ends_with(SO_FAR), "{shown}");
    ensure!(
        shown.chars().count() <= SHOWN_CHARS,
        "{} characters",
        shown.chars().count()
    );
    let size = pill_size(world)?;
    ensure!(
        run(world).sizes == [size],
        "the pill went from {:?} to {size:?}",
        run(world).sizes
    );
    Ok(())
}

#[then("the pill has one size in all of them")]
fn one_size(world: &mut IntegrationWorld) -> Result<()> {
    let sizes = &run(world).sizes;
    ensure!(
        sizes.len() == 3 && sizes.windows(2).all(|w| w[0] == w[1]),
        "{sizes:?}"
    );
    Ok(())
}

#[then("the glass takes no keyboard focus and has no line along its top edge")]
fn glass_finish(world: &mut IntegrationWorld) -> Result<()> {
    let overlay = overlay(world)?;
    ensure!(
        !overlay.has_key_window(),
        "the glass takes the keyboard focus"
    );
    let finishes = overlay.first_glass_finishes();
    ensure!(!finishes.is_empty(), "no glass shows");
    ensure!(
        finishes.iter().all(|f| !f.top_highlight),
        "a top line: {finishes:?}"
    );
    let clear = finishes
        .iter()
        .all(|f| f.clear_system_glass.unwrap_or(f.own_rim));
    ensure!(
        clear,
        "the glass is not the clear system glass: {finishes:?}"
    );
    Ok(())
}

/// Whether the pill shows the muted microphone, and the record sign when `recording`.
fn muted(world: &mut IntegrationWorld, recording: bool) -> Result<&MacosVisualIndicator> {
    let overlay = overlay(world)?;
    let shown = (
        overlay.first_shows_commands_unavailable(),
        overlay.first_shows_recording(),
    );
    ensure!(
        shown == (true, recording),
        "muted microphone and record sign: {shown:?}"
    );
    Ok(overlay)
}

#[then(expr = "the pill shows the muted microphone with the label {string}")]
fn muted_microphone_alone(world: &mut IntegrationWorld, label: String) -> Result<()> {
    let shown = muted(world, false)?.first_label_value();
    ensure!(shown == Some(label), "the pill reads {shown:?}");
    Ok(())
}

#[then("the pill shows the muted microphone beside the record sign")]
fn muted_beside_record_sign(world: &mut IntegrationWorld) -> Result<()> {
    muted(world, true).map(drop)
}
