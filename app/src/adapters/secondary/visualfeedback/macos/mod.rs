mod failure_notice;
mod glass;
mod live_text;
mod motion;
mod pill_content;
mod pill_states;
mod pill_view;
mod pointer_sign;
mod screen_watcher;
mod sign_badge;
mod symbols;
mod topology;
mod transcript_window;
mod waveform;
mod window_set;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSScreen};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::core::application::{AudioLevelSource, LiveTranscriptView, VisualIndicator};
use crate::core::domain::{AudioLevel, FailureNotice, VoiceCommand};

use crate::adapters::main_queue::{dispatch_to_main, dispatch_to_main_after};
use failure_notice::{show_notice, NoticeTimer};
use live_text::LiveTranscriptText;
use pill_content::{command_sign, state_after_sign};
use pill_states::{
    apply_command_sign, apply_hidden_state, apply_pasted_sign, apply_processing_state,
    apply_recording_state, apply_waiting_for_focus_state, fade_out_then_hide, LevelFeed, Turn,
    Windows,
};
use screen_watcher::ScreenWatcher;
use topology::{diff_screens, ScreenChange, ScreenTopology};
use waveform::LevelWave;
use window_set::WindowSet;

pub use glass::GlassFinish;
pub use pointer_sign::MacosPointerSign;

use super::pointer_sign::{FAILURE_NOTICE_LIFETIME, PASTED_SIGN_LIFETIME};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    RecognizingSpeech,
    RefiningText,
    WaitingForFocus,
    FailureNotice(FailureNotice),
}

/// How long a command sign stays before the overlay shows again what it showed before.
const COMMAND_SIGN_MS: i64 = 1_000;
/// How long the Pasted sign shows before the overlay fades out.
const PASTED_SIGN_MS: i64 = PASTED_SIGN_LIFETIME.as_millis() as i64;

/// How long a failure notice stays before the overlay hides.
const NOTICE_DURATION: Duration = FAILURE_NOTICE_LIFETIME;

/// The screen the overlay draws on when macOS reports none.
const FALLBACK_SCREEN: NSRect =
    NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { width: 1440.0, height: 900.0 } };

type SharedWindows = Arc<Mutex<Windows>>;

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    /// Whether the commands-unavailable sign shows beside the state.
    commands_unavailable: Arc<AtomicBool>,
    windows: SharedWindows,
    pulse_generation: Arc<AtomicU64>,
    feed: LevelFeed,
    live_transcript: LiveTranscriptText,
    last_topology: Arc<Mutex<ScreenTopology>>,
    notice_duration_ms: i64,
    _watcher: ScreenWatcher,
}

impl MacosVisualIndicator {
    pub fn initialize(mtm: MainThreadMarker, level_register: Arc<AtomicU32>) -> Self {
        Self::initialize_with_notice_duration(mtm, level_register, NOTICE_DURATION)
    }

    /// The overlay whose failure notice hides `notice_duration` after it is shown.
    pub fn initialize_with_notice_duration(
        mtm: MainThreadMarker,
        level_register: Arc<AtomicU32>,
        notice_duration: Duration,
    ) -> Self {
        let notice_duration_ms = i64::try_from(notice_duration.as_millis()).unwrap_or(i64::MAX);
        let windows = Arc::new(Mutex::new(build_windows(mtm)));
        let last_topology = Arc::new(Mutex::new(collect_topology(mtm)));

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let state = Arc::new(Mutex::new(OverlayState::Hidden));
        let commands_unavailable = Arc::new(AtomicBool::new(false));
        let pulse_generation = Arc::new(AtomicU64::new(0));
        let redraw = Redraw {
            windows,
            feed: LevelFeed {
                register: level_register,
                wave: Arc::new(Mutex::new(LevelWave::starting_now())),
            },
            live_transcript: LiveTranscriptText::default(),
            timer: NoticeTimer {
                duration_ms: notice_duration_ms,
                pulse_generation: Arc::clone(&pulse_generation),
                state: Arc::clone(&state),
                commands_unavailable: Arc::clone(&commands_unavailable),
            },
        };
        let watcher = ScreenWatcher::install(screen_change_handler(
            redraw.clone(),
            Arc::clone(&last_topology),
        ));
        Self {
            state,
            commands_unavailable,
            windows: redraw.windows,
            pulse_generation,
            feed: redraw.feed,
            live_transcript: redraw.live_transcript,
            last_topology,
            notice_duration_ms,
            _watcher: watcher,
        }
    }

    /// Rebuilds the windows when the screens changed and redraws the current state.
    pub fn rebuild_window_set(&self, mtm: MainThreadMarker) {
        if diff_screens(&self.last_topology.lock().unwrap(), &collect_topology(mtm))
            == ScreenChange::Identical
        {
            return;
        }
        let redraw = self.redraw();
        let turn = Turn::next(&redraw.timer.pulse_generation);
        do_rebuild(mtm, &redraw.windows, &self.last_topology);
        let current_state = *redraw.timer.state.lock().unwrap();
        redraw.draw(current_state, turn);
    }

    pub fn generation_count(&self) -> u64 {
        self.pulse_generation.load(Ordering::SeqCst)
    }

    /// The symbol of the state's icon on the pill of the first screen, while one shows.
    pub fn first_icon_symbol(&self) -> Option<String> {
        self.first_window()?.pill.icon_symbol()
    }

    /// The label on the pill of the first screen, while one shows.
    pub fn first_label_value(&self) -> Option<String> {
        self.first_window()?.pill.label_text()
    }

    /// Whether the pill of the first screen shows the record sign and the waveform.
    pub fn first_shows_recording(&self) -> bool {
        self.first_window()
            .is_some_and(|ws| ws.pill.shows_record_sign())
    }

    /// Whether the pill of the first screen shows the muted microphone.
    pub fn first_shows_commands_unavailable(&self) -> bool {
        self.first_window()
            .is_some_and(|ws| ws.pill.shows_commands_unavailable())
    }

    /// The tallest waveform column on the pill of the first screen, in points.
    pub fn first_tallest_waveform_column(&self) -> f64 {
        self.first_window()
            .map_or(0.0, |ws| ws.pill.waveform.tallest_column())
    }

    /// Whether the record dot pulses or the state's icon breathes on the first screen.
    pub fn first_shows_motion(&self) -> bool {
        self.first_window()
            .is_some_and(|ws| ws.pill.record_dot_pulses() || ws.pill.icon_breathes())
    }

    /// The text the window under the pill of the first screen shows; `None` while it is hidden.
    pub fn first_text_window_value(&self) -> Option<String> {
        let ws = self.first_window()?;
        let transcript = &ws.transcript;
        transcript
            .window()
            .isVisible()
            .then(|| transcript.label.stringValue().to_string())
    }

    /// Where the glass of the pill of the first screen is, in screen coordinates, while it shows.
    pub fn first_pill_glass_frame(&self) -> Option<NSRect> {
        let ws = self.first_window()?;
        ws.window().isVisible().then(|| ws.glass_rect())
    }

    /// Where the glass of the text window of the first screen is, in screen coordinates,
    /// while it shows.
    pub fn first_text_window_glass_frame(&self) -> Option<NSRect> {
        let ws = self.first_window()?;
        let transcript = &ws.transcript;
        transcript
            .window()
            .isVisible()
            .then(|| transcript.glass_rect())
    }

    /// How the glass of the pill and of the text window of the first screen is finished.
    pub fn first_glass_finishes(&self) -> Vec<GlassFinish> {
        self.first_window().map_or_else(Vec::new, |ws| {
            vec![ws.glass_finish(), ws.transcript.glass_finish()]
        })
    }

    /// Whether the pill of the first screen is on screen.
    pub fn is_pill_visible(&self) -> bool {
        self.first_window()
            .is_some_and(|ws| ws.window().isVisible())
    }

    /// Whether any overlay window took the keyboard focus or could take it.
    pub fn has_key_window(&self) -> bool {
        self.windows.lock().unwrap().iter().any(|ws| {
            [ws.window(), ws.transcript.window()]
                .iter()
                .any(|window| window.isKeyWindow() || window.canBecomeKeyWindow())
        })
    }

    fn first_window(&self) -> Option<Arc<WindowSet>> {
        self.windows.lock().unwrap().first().cloned()
    }

    fn notice_timer(&self) -> NoticeTimer {
        NoticeTimer {
            duration_ms: self.notice_duration_ms,
            pulse_generation: Arc::clone(&self.pulse_generation),
            state: Arc::clone(&self.state),
            commands_unavailable: Arc::clone(&self.commands_unavailable),
        }
    }

    fn redraw(&self) -> Redraw {
        Redraw {
            windows: Arc::clone(&self.windows),
            feed: self.feed.clone(),
            live_transcript: self.live_transcript.clone(),
            timer: self.notice_timer(),
        }
    }

    /// Sets `state` and draws it on freshly built windows, on the main queue.
    fn show_state(&self, state: OverlayState) {
        *self.state.lock().unwrap() = state;
        log::debug!("overlay: {state:?} requested");
        let turn = Turn::next(&self.pulse_generation);
        let redraw = self.redraw();
        let last_topology = Arc::clone(&self.last_topology);
        dispatch_to_main(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                do_rebuild(mtm, &redraw.windows, &last_topology);
                redraw.draw(state, turn);
            }
        });
    }

    fn hide_overlay(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let windows = Arc::clone(&self.windows);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *self.state.lock().unwrap() = OverlayState::Hidden;
        log::debug!("overlay: hide requested");

        dispatch_to_main(move || {
            log::debug!("overlay: hidden");
            // WHY: read on the main queue; a show queued before this hide may build the windows
            let windows = windows.lock().unwrap().clone();
            apply_hidden_state(windows, commands_unavailable.load(Ordering::SeqCst));
        });
    }

    /// Redraws the state shown with or without the commands-unavailable sign; a failure notice
    /// keeps showing alone, and its end draws the state with the sign.
    fn set_commands_unavailable(&self, unavailable: bool) {
        self.commands_unavailable
            .store(unavailable, Ordering::SeqCst);
        let redraw = self.redraw();
        log::debug!("overlay: commands unavailable {unavailable} requested");

        dispatch_to_main(move || {
            // WHY: read on the main queue, so a state shown after this signal was queued wins
            let state = *redraw.timer.state.lock().unwrap();
            if matches!(state, OverlayState::FailureNotice(_)) {
                return;
            }
            let turn = Turn::next(&redraw.timer.pulse_generation);
            redraw.draw(state, turn);
        });
    }
}

/// Rebuilds the windows for the screens of the moment and redraws the current state.
fn screen_change_handler(
    redraw: Redraw,
    last_topology: Arc<Mutex<ScreenTopology>>,
) -> Arc<dyn Fn() + Send + Sync + 'static> {
    Arc::new(move || {
        let redraw = redraw.clone();
        let last_topology = Arc::clone(&last_topology);
        dispatch_to_main(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let turn = Turn::next(&redraw.timer.pulse_generation);
            do_rebuild(mtm, &redraw.windows, &last_topology);
            let current_state = *redraw.timer.state.lock().unwrap();
            redraw.draw(current_state, turn);
        });
    })
}

fn build_windows(mtm: MainThreadMarker) -> Windows {
    let screens = NSScreen::screens(mtm);
    if screens.is_empty() {
        return vec![WindowSet::build(mtm, FALLBACK_SCREEN)];
    }
    (0..screens.count())
        .map(|i| WindowSet::build(mtm, screens.objectAtIndex(i).visibleFrame()))
        .collect()
}

fn do_rebuild(
    mtm: MainThreadMarker,
    windows: &SharedWindows,
    last_topology: &Arc<Mutex<ScreenTopology>>,
) {
    let new_topology = collect_topology(mtm);
    let new_windows = build_windows(mtm);
    let old_windows = std::mem::replace(&mut *windows.lock().unwrap(), new_windows);
    drop(old_windows);
    *last_topology.lock().unwrap() = new_topology;
}

fn collect_topology(mtm: MainThreadMarker) -> ScreenTopology {
    let screens = NSScreen::screens(mtm);
    let frames: Vec<NSRect> = (0..screens.count())
        .map(|i| screens.objectAtIndex(i).visibleFrame())
        .collect();
    ScreenTopology { frames }
}

/// What draws a state of the overlay on the windows of the moment, with the
/// commands-unavailable sign while it is on.
#[derive(Clone)]
struct Redraw {
    windows: SharedWindows,
    feed: LevelFeed,
    live_transcript: LiveTranscriptText,
    timer: NoticeTimer,
}

impl Redraw {
    fn draw(&self, state: OverlayState, turn: Turn) {
        let commands_unavailable = self.timer.commands_unavailable.load(Ordering::SeqCst);
        let ws_vec = self.windows.lock().unwrap().clone();
        log::debug!("overlay: {state:?} shown on {} screen(s)", ws_vec.len());
        match state {
            OverlayState::Recording => {
                let shown = self.live_transcript.shown(state);
                apply_recording_state(
                    ws_vec.clone(),
                    turn,
                    self.feed.clone(),
                    commands_unavailable,
                );
                apply_live_transcript(&ws_vec, shown);
            }
            OverlayState::RecognizingSpeech | OverlayState::RefiningText => {
                apply_processing_state(ws_vec, state, turn, commands_unavailable);
            }
            OverlayState::WaitingForFocus => {
                apply_waiting_for_focus_state(ws_vec, turn, commands_unavailable);
            }
            OverlayState::FailureNotice(notice) => {
                show_notice(&self.windows, &notice, turn.generation, self.timer.clone());
            }
            OverlayState::Hidden => apply_hidden_state(ws_vec, commands_unavailable),
        }
    }
}

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        self.live_transcript.clear();
        *self.feed.wave.lock().unwrap() = LevelWave::starting_now();
        self.show_state(OverlayState::Recording);
    }

    fn show_recognizing_speech(&self) {
        self.show_state(OverlayState::RecognizingSpeech);
    }

    fn show_refining_text(&self) {
        self.show_state(OverlayState::RefiningText);
    }

    fn show_waiting_for_focus(&self) {
        self.show_state(OverlayState::WaitingForFocus);
    }

    fn hide(&self) {
        self.hide_overlay();
    }

    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    fn show_pasted(&self) {
        let turn = Turn::next(&self.pulse_generation);
        let windows = Arc::clone(&self.windows);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);
        // The sign hides itself, so a screen change draws the overlay hidden.
        *self.state.lock().unwrap() = OverlayState::Hidden;
        log::debug!("overlay: Pasted sign requested");

        dispatch_to_main(move || {
            if !turn.is_current() {
                return;
            }
            let ws_vec = windows.lock().unwrap().clone();
            apply_pasted_sign(&ws_vec);
            dispatch_to_main_after(PASTED_SIGN_MS, move || {
                // WHY: a later signal took over the overlay; its own drawing stands
                if turn.is_current() {
                    fade_out_then_hide(ws_vec, turn, commands_unavailable);
                }
            });
        });
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.show_state(OverlayState::FailureNotice(*notice));
    }

    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    fn show_command_sign(&self, command: VoiceCommand) {
        let Some(sign) = command_sign(command) else {
            return;
        };
        let turn = Turn::next(&self.pulse_generation);
        let redraw = self.redraw();
        let last_topology = Arc::clone(&self.last_topology);
        // The state stays what the command left, so a screen change redraws that state.
        let after = state_after_sign(*self.state.lock().unwrap());
        log::debug!("overlay: command sign {} requested", sign.label);

        dispatch_to_main(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            do_rebuild(mtm, &redraw.windows, &last_topology);
            apply_command_sign(&redraw.windows.lock().unwrap(), sign);
            dispatch_to_main_after(COMMAND_SIGN_MS, move || {
                // WHY: a later signal took over the overlay; its own drawing stands
                if turn.is_current() {
                    redraw.draw(after, turn);
                }
            });
        });
    }

    fn show_commands_unavailable(&self) {
        self.set_commands_unavailable(true);
    }

    fn hide_commands_unavailable(&self) {
        self.set_commands_unavailable(false);
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(MacosLiveTranscriptView {
            state: Arc::clone(&self.state),
            windows: Arc::clone(&self.windows),
            live_transcript: self.live_transcript.clone(),
        })
    }
}

/// Draws the live transcript under the pill of every screen while the recording runs.
struct MacosLiveTranscriptView {
    state: Arc<Mutex<OverlayState>>,
    windows: SharedWindows,
    live_transcript: LiveTranscriptText,
}

impl LiveTranscriptView for MacosLiveTranscriptView {
    fn show_live_transcript(&self, text: &str) {
        self.live_transcript
            .update(*self.state.lock().unwrap(), text);
        let state = Arc::clone(&self.state);
        let windows = Arc::clone(&self.windows);
        let live_transcript = self.live_transcript.clone();
        dispatch_to_main(move || {
            // WHY: read on the main queue, so a stage shown after this update was queued wins
            let state = *state.lock().unwrap();
            if LiveTranscriptText::draws_text_window(state) {
                apply_live_transcript(&windows.lock().unwrap(), live_transcript.shown(state));
            }
        });
    }
}

fn apply_live_transcript(windows: &[Arc<WindowSet>], shown: Option<String>) {
    for ws in windows {
        ws.show_live_transcript(shown.as_deref());
    }
}

impl AudioLevelSource for MacosVisualIndicator {
    fn read_level(&self) -> AudioLevel {
        AudioLevel::from_bits(self.feed.register.load(Ordering::Relaxed))
    }
}

unsafe impl Send for MacosVisualIndicator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_level_from_register_via_audio_level_source() {
        let reg = Arc::new(AtomicU32::new(0.5f32.to_bits()));
        let level = AudioLevel::from_bits(reg.load(Ordering::Relaxed));
        assert!((level.value() - 0.5).abs() < 1e-6);
    }
}
