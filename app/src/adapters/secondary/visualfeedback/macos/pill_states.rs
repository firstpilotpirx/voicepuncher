//! Draws each state of the overlay on the windows of every screen, on the main queue, with
//! the timed work that keeps it alive: the heartbeat, the waveform tick and the Pasted fade.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use objc2::MainThreadMarker;

use crate::core::domain::level_history::{CAPACITY_BESIDE_SIGN, FULL_CAPACITY};
use crate::core::domain::AudioLevel;

use super::motion::reduce_motion;
use super::pill_content::{state_sign, GlassTint, PillKind, SignLook, PASTED_SIGN};
use super::pill_view::PillLook;
use super::waveform::LevelWave;
use super::window_set::{SharedWindows, WindowSet};
use super::OverlayState;
use crate::adapters::locking::lock;
use crate::adapters::main_queue::on_main_after;

/// How often the overlay is brought back over other windows.
const HEARTBEAT_MS: i64 = 200;
/// How often the waveform takes the level and redraws: about once per frame.
const WAVEFORM_TICK_MS: i64 = 16;
/// How long the Pasted sign fades out before the overlay hides.
const PASTED_FADE_SECS: f64 = 0.2;
const FADE_FRAME_MS: i64 = 16;

const RECORDING_LOOK: PillLook =
    PillLook { kind: PillKind::Recording, sign: None, breathes: false };

/// One drawing of the overlay; every later signal bumps the counter, which ends the timed work
/// of this one.
#[derive(Clone)]
pub(super) struct Turn {
    pub(super) counter: Arc<AtomicU64>,
    pub(super) generation: u64,
}

impl Turn {
    /// Starts a new drawing on `counter`.
    pub(super) fn next(counter: &Arc<AtomicU64>) -> Self {
        Self {
            counter: Arc::clone(counter),
            generation: counter.fetch_add(1, Ordering::SeqCst) + 1,
        }
    }

    pub(super) fn is_current(&self) -> bool {
        self.counter.load(Ordering::SeqCst) == self.generation
    }
}

/// What feeds the recording's waveform: the microphone level and its history.
#[derive(Clone)]
pub(super) struct LevelFeed {
    pub(super) register: Arc<AtomicU32>,
    pub(super) wave: Arc<Mutex<LevelWave>>,
}

/// How many columns the waveform shows: fewer beside the commands-unavailable sign.
fn visible_columns(commands_unavailable: bool) -> usize {
    if commands_unavailable {
        CAPACITY_BESIDE_SIGN
    } else {
        FULL_CAPACITY
    }
}

/// The record sign and the waveform; the text window stays as it is.
pub(super) fn apply_recording_state(
    main: &SharedWindows,
    mtm: MainThreadMarker,
    turn: Turn,
    feed: LevelFeed,
    commands_unavailable: bool,
) {
    let columns = visible_columns(commands_unavailable);
    let sliding = !reduce_motion();
    {
        let wave = lock(&feed.wave);
        for ws in &main.windows(mtm) {
            ws.draw(
                RECORDING_LOOK,
                GlassTint::Clear,
                commands_unavailable,
                columns,
            );
            // WHY: the first tick comes a frame later; the record sign never shows beside an empty strip
            ws.pill
                .waveform
                .draw(&wave, wave.elapsed(), columns, sliding);
        }
    }
    schedule_heartbeat(main.clone(), turn.clone());
    schedule_waveform_tick(main.clone(), turn, feed, columns, sliding);
}

/// The stage's breathing icon, its label and the spinner; no text window.
pub(super) fn apply_processing_state(
    (main, mtm): (&SharedWindows, MainThreadMarker),
    stage: OverlayState,
    turn: Turn,
    commands_unavailable: bool,
) {
    let look = PillLook { kind: PillKind::Processing, sign: state_sign(stage), breathes: true };
    for ws in &main.windows(mtm) {
        ws.draw(look, GlassTint::Clear, commands_unavailable, FULL_CAPACITY);
        ws.transcript.hide();
    }
    schedule_heartbeat(main.clone(), turn);
}

/// Orange glass with the breathing keyboard; no text window.
pub(super) fn apply_waiting_for_focus_state(
    (main, mtm): (&SharedWindows, MainThreadMarker),
    turn: Turn,
    commands_unavailable: bool,
) {
    let look = PillLook {
        kind: PillKind::Sign,
        sign: state_sign(OverlayState::WaitingForFocus),
        breathes: true,
    };
    for ws in &main.windows(mtm) {
        ws.draw(
            look,
            GlassTint::Waiting,
            commands_unavailable,
            FULL_CAPACITY,
        );
        ws.transcript.hide();
    }
    schedule_heartbeat(main.clone(), turn);
}

/// The green check and "Pasted"; no text window.
pub(super) fn apply_pasted_sign(windows: &[Rc<WindowSet>]) {
    let look = PillLook { kind: PillKind::Sign, sign: Some(PASTED_SIGN), breathes: false };
    for ws in windows {
        ws.draw(look, GlassTint::Clear, false, FULL_CAPACITY);
        ws.transcript.hide();
    }
}

/// The command's icon and name in place of the state's sign, steady; the text window stays
/// as it is.
pub(super) fn apply_command_sign(windows: &[Rc<WindowSet>], sign: SignLook) {
    let look = PillLook { kind: PillKind::Command, sign: Some(sign), breathes: false };
    for ws in windows {
        ws.draw(look, GlassTint::Clear, false, FULL_CAPACITY);
    }
}

/// Hides the overlay, or leaves the commands-unavailable sign alone in it while it is on.
pub(super) fn apply_hidden_state(windows: &[Rc<WindowSet>], commands_unavailable: bool) {
    if commands_unavailable {
        apply_commands_unavailable_alone(windows);
        return;
    }
    for ws in windows {
        ws.hide();
    }
}

/// The muted microphone and "CMD" alone in a compact pill.
fn apply_commands_unavailable_alone(windows: &[Rc<WindowSet>]) {
    let look = PillLook { kind: PillKind::UnavailableAlone, sign: None, breathes: false };
    for ws in windows {
        ws.draw(look, GlassTint::Clear, true, FULL_CAPACITY);
        ws.transcript.hide();
    }
}

/// Fades the overlay out, then hides it, unless a later signal takes the overlay first.
pub(super) fn fade_out_then_hide(
    main: SharedWindows,
    turn: Turn,
    commands_unavailable: Arc<AtomicBool>,
) {
    step_fade(main, turn, commands_unavailable, Instant::now());
}

fn step_fade(
    main: SharedWindows,
    turn: Turn,
    commands_unavailable: Arc<AtomicBool>,
    start: Instant,
) {
    on_main_after(FADE_FRAME_MS, move |mtm| {
        // WHY: a later signal took over the overlay; its own drawing stands
        if !turn.is_current() {
            return;
        }
        let progress = (start.elapsed().as_secs_f64() / PASTED_FADE_SECS).min(1.0);
        let windows = main.windows(mtm);
        if progress < 1.0 {
            for ws in &windows {
                ws.set_opacity(1.0 - progress);
            }
            step_fade(main, turn, commands_unavailable, start);
            return;
        }
        apply_hidden_state(&windows, commands_unavailable.load(Ordering::SeqCst));
    });
}

/// Brings the overlay back over other windows every heartbeat while `turn` lasts.
pub(super) fn schedule_heartbeat(main: SharedWindows, turn: Turn) {
    on_main_after(HEARTBEAT_MS, move |mtm| {
        if !turn.is_current() {
            return;
        }
        for ws in &main.windows(mtm) {
            ws.keep_in_front();
        }
        schedule_heartbeat(main, turn);
    });
}

/// Feeds the waveform the microphone level and redraws it every frame while `turn` lasts.
fn schedule_waveform_tick(
    main: SharedWindows,
    turn: Turn,
    feed: LevelFeed,
    columns: usize,
    sliding: bool,
) {
    on_main_after(WAVEFORM_TICK_MS, move |mtm| {
        if !turn.is_current() {
            return;
        }
        let level = AudioLevel::from_bits(feed.register.load(Ordering::Relaxed));
        {
            let mut wave = lock(&feed.wave);
            let at = wave.record(level);
            for ws in &main.windows(mtm) {
                ws.pill.waveform.draw(&wave, at, columns, sliding);
            }
        }
        schedule_waveform_tick(main, turn, feed, columns, sliding);
    });
}
