mod failure_notice;
mod glass;
mod live_text;
mod motion;
mod pill_content;
mod pill_states;
mod pill_view;
mod pointer_sign;
mod sign_badge;
mod symbols;
mod topology;
mod transcript_window;
mod waveform;
mod window_set;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSScreen};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::core::application::{AudioLevelSource, LiveTranscriptView, OnDrawn, VisualIndicator};
use crate::core::domain::{AudioLevel, FailureNotice, VoiceCommand};

use crate::adapters::locking::lock;
use crate::adapters::main_queue::{on_main, on_main_after, MainOnly};
use crate::adapters::screen_watcher::ScreenWatcher;
use failure_notice::{show_notice, NoticeTimer};
use live_text::LiveTranscriptText;
use pill_content::{command_sign, state_after_sign};
use pill_states::{
    apply_command_sign, apply_hidden_state, apply_pasted_sign, apply_processing_state,
    apply_recording_state, apply_waiting_for_focus_state, fade_out_then_hide, LevelFeed, Turn,
};
use topology::{diff_screens, ScreenChange, ScreenTopology};
use waveform::LevelWave;
use window_set::{MainState, SharedWindows, WindowSet, Windows};

pub use glass::GlassFinish;
pub use pointer_sign::MacosPointerSign;

use super::pointer_sign::{FAILURE_NOTICE_LIFETIME, PASTED_SIGN_MS};

mod redraw;
use redraw::{
    build_windows, collect_topology, rebuild_if_screens_changed, screen_change_handler, Redraw,
};
mod indicator;
mod live_view;
use live_view::{apply_live_transcript, MacosLiveTranscriptView};

#[derive(Debug, Clone, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    RecognizingSpeech,
    WaitingForFocus,
    FailureNotice(FailureNotice),
}

/// How long a command sign stays before the overlay shows again what it showed before.
const COMMAND_SIGN_MS: i64 = 1_000;

/// How long a failure notice stays before the overlay hides.
const NOTICE_DURATION: Duration = FAILURE_NOTICE_LIFETIME;

/// The screen the overlay draws on when macOS reports none.
const FALLBACK_SCREEN: NSRect =
    NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { width: 1440.0, height: 900.0 } };

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    /// Whether the commands-unavailable sign shows beside the state.
    commands_unavailable: Arc<AtomicBool>,
    main: SharedWindows,
    pulse_generation: Arc<AtomicU64>,
    feed: LevelFeed,
    live_transcript: LiveTranscriptText,
    notice_duration_ms: i64,
    _watcher: MainOnly<ScreenWatcher>,
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
        let main = MainState {
            windows: RefCell::new(build_windows(mtm)),
            topology: RefCell::new(collect_topology(mtm)),
        };
        let main = MainOnly::new(mtm, main);

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let state = Arc::new(Mutex::new(OverlayState::Hidden));
        let commands_unavailable = Arc::new(AtomicBool::new(false));
        let pulse_generation = Arc::new(AtomicU64::new(0));
        let redraw = Redraw {
            main,
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
        let watcher = ScreenWatcher::install(screen_change_handler(redraw.clone()));
        Self {
            state,
            commands_unavailable,
            main: redraw.main,
            pulse_generation,
            feed: redraw.feed,
            live_transcript: redraw.live_transcript,
            notice_duration_ms,
            _watcher: MainOnly::new(mtm, watcher),
        }
    }

    /// Rebuilds the windows when the screens changed and redraws the current state.
    pub fn rebuild_window_set(&self, mtm: MainThreadMarker) {
        let Some(main) = self.main.get(mtm) else {
            return;
        };
        if diff_screens(&main.topology.borrow(), &collect_topology(mtm)) == ScreenChange::Identical
        {
            return;
        }
        self.redraw().rebuild_and_draw(mtm);
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

    /// How many times the overlay's windows were built in this process.
    pub fn window_builds(&self) -> u64 {
        redraw::WINDOW_BUILDS.load(Ordering::SeqCst)
    }

    /// Forgets the screens the windows were built for, as when they changed unnoticed.
    pub fn forget_the_screens(&self, mtm: MainThreadMarker) {
        if let Some(main) = self.main.get(mtm) {
            main.topology.replace(ScreenTopology { frames: Vec::new() });
        }
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
        self.windows_here().iter().any(|ws| {
            [ws.window(), ws.transcript.window()]
                .iter()
                .any(|window| window.isKeyWindow() || window.canBecomeKeyWindow())
        })
    }

    fn first_window(&self) -> Option<Rc<WindowSet>> {
        self.windows_here().first().cloned()
    }

    /// The windows, read on the main thread; none elsewhere.
    fn windows_here(&self) -> Windows {
        self.main
            .on_this_thread()
            .map(|main| main.windows.borrow().clone())
            .unwrap_or_default()
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
            main: self.main.clone(),
            feed: self.feed.clone(),
            live_transcript: self.live_transcript.clone(),
            timer: self.notice_timer(),
        }
    }

    /// Sets `state` and draws it on the main queue, on windows built again only for changed
    /// screens.
    fn show_state(&self, state: OverlayState) {
        self.show_state_then(state, Box::new(|_| {}));
    }

    /// Sets `state`, draws it on the main queue and tells `on_drawn` when it was drawn.
    fn show_state_then(&self, state: OverlayState, on_drawn: OnDrawn) {
        *lock(&self.state) = state.clone();
        log::debug!("overlay: {state:?} requested");
        let turn = Turn::next(&self.pulse_generation);
        let redraw = self.redraw();
        on_main(move |mtm| {
            rebuild_if_screens_changed(mtm, &redraw.main);
            redraw.draw(mtm, &state, turn);
            on_drawn(Instant::now());
        });
    }

    fn hide_overlay(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let main = self.main.clone();
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *lock(&self.state) = OverlayState::Hidden;
        log::debug!("overlay: hide requested");

        on_main(move |mtm| {
            log::debug!("overlay: hidden");
            // WHY: read on the main queue; a show queued before this hide may build the windows
            apply_hidden_state(
                &main.windows(mtm),
                commands_unavailable.load(Ordering::SeqCst),
            );
        });
    }

    /// Redraws the state shown with or without the commands-unavailable sign; a failure notice
    /// keeps showing alone, and its end draws the state with the sign.
    fn set_commands_unavailable(&self, unavailable: bool) {
        self.commands_unavailable
            .store(unavailable, Ordering::SeqCst);
        let redraw = self.redraw();
        log::debug!("overlay: commands unavailable {unavailable} requested");

        on_main(move |mtm| {
            // WHY: read on the main queue, so a state shown after this signal was queued wins
            let state = lock(&redraw.timer.state).clone();
            if matches!(state, OverlayState::FailureNotice(_)) {
                return;
            }
            let turn = Turn::next(&redraw.timer.pulse_generation);
            redraw.draw(mtm, &state, turn);
        });
    }
}

impl AudioLevelSource for MacosVisualIndicator {
    fn read_level(&self) -> AudioLevel {
        AudioLevel::from_bits(self.feed.register.load(Ordering::Relaxed))
    }
}

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
