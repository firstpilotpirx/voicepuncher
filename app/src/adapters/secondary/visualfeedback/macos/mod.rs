mod failure_notice;
mod live_text;
mod pointer_sign;
mod screen_watcher;
mod topology;
mod transcript_window;
mod window_set;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSScreen};

use crate::core::application::{AudioLevelSource, LiveTranscriptView, VisualIndicator};
use crate::core::domain::{level_to_pills, AudioLevel, FailureNotice, VoiceCommand};

use failure_notice::{show_notice, NoticeTimer};
use live_text::LiveTranscriptText;
use screen_watcher::ScreenWatcher;
use topology::{diff_screens, ScreenChange, ScreenTopology};
use window_set::{PillWidth, WindowSet};

pub use pointer_sign::MacosPointerSign;

type PulseData = (Vec<Arc<WindowSet>>, u64, Arc<AtomicU64>, bool);
type HeartbeatData = (Vec<Arc<WindowSet>>, u64, Arc<AtomicU64>);
type EqualizerData = (Vec<Arc<WindowSet>>, u64, Arc<AtomicU64>, Arc<AtomicU32>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    RecognizingSpeech,
    RefiningText,
    WaitingForFocus,
    FailureNotice(FailureNotice),
}

const RECORDING_SIGN: &str = "◉ REC";
const WAITING_FOR_FOCUS_SIGN: &str = "⌨ FOCUS";
const COMMANDS_UNAVAILABLE_SIGN: &str = "⊘ CMD";

/// How long a command sign stays before the overlay shows again what it showed before.
const COMMAND_SIGN_MS: i64 = 1_000;

/// How long a failure notice stays before the overlay hides.
const NOTICE_DURATION: Duration = Duration::from_secs(4);

/// A command sign: its icon in the pill's sign place, its name where the level shows.
type CommandSign = (&'static str, &'static str);

/// The command sign of a voice command with no visible effect of its own; `None` for a
/// command the overlay already shows (start, stop, cancel).
fn command_sign(command: VoiceCommand) -> Option<CommandSign> {
    match command {
        VoiceCommand::PressEnter => Some(("⏎", "ENTER")),
        VoiceCommand::ClearAllText => Some(("⌫", "CLEAR")),
        VoiceCommand::PasteText => Some(("⎘", "PASTE")),
        VoiceCommand::RestartDictation => Some(("↻", "RESTART")),
        VoiceCommand::StartDictation
        | VoiceCommand::StopDictation
        | VoiceCommand::CancelDictation => None,
    }
}

/// What the overlay shows once a command sign shown in `state` is over: the recording goes
/// on, anything else was already over when the command ran.
fn state_after_sign(state: OverlayState) -> OverlayState {
    match state {
        OverlayState::Recording => OverlayState::Recording,
        _ => OverlayState::Hidden,
    }
}

/// The icon of a processing stage; `None` for a state that is not one.
fn stage_icon(state: OverlayState) -> Option<&'static str> {
    match state {
        OverlayState::RecognizingSpeech => Some("🎧"),
        OverlayState::RefiningText => Some("✨"),
        _ => None,
    }
}

/// The sign the pill shows in `state`, followed by the commands-unavailable sign while
/// `commands_unavailable`; `None` when the pill is hidden. A failure notice draws its own sign.
fn pill_sign(state: OverlayState, commands_unavailable: bool) -> Option<String> {
    let state_sign = match state {
        OverlayState::Recording => Some(RECORDING_SIGN),
        OverlayState::RecognizingSpeech | OverlayState::RefiningText => stage_icon(state),
        OverlayState::WaitingForFocus => Some(WAITING_FOR_FOCUS_SIGN),
        OverlayState::Hidden => None,
        OverlayState::FailureNotice(_) => return None,
    };
    match (state_sign, commands_unavailable) {
        (Some(sign), true) => Some(format!("{sign} {COMMANDS_UNAVAILABLE_SIGN}")),
        (Some(sign), false) => Some(sign.to_owned()),
        (None, true) => Some(COMMANDS_UNAVAILABLE_SIGN.to_owned()),
        (None, false) => None,
    }
}

/// How wide the pill is for a state drawn with or without the commands-unavailable sign.
fn pill_width(commands_unavailable: bool) -> PillWidth {
    if commands_unavailable {
        PillWidth::FitSign
    } else {
        PillWidth::Base
    }
}

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    static _dispatch_main_q: std::ffi::c_void;
    fn dispatch_async_f(
        queue: *const std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: unsafe extern "C" fn(*mut std::ffi::c_void),
    );
    fn dispatch_after_f(
        when: u64,
        queue: *const std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: unsafe extern "C" fn(*mut std::ffi::c_void),
    );
    fn dispatch_time(when: u64, delta: i64) -> u64;
}

const DISPATCH_TIME_NOW: u64 = 0;
const NSEC_PER_SEC: i64 = 1_000_000_000;

fn main_queue() -> *const std::ffi::c_void {
    &raw const _dispatch_main_q
}

fn dispatch_to_main<F: FnOnce() + Send + 'static>(f: F) {
    let boxed: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(f));
    let ptr = Box::into_raw(boxed) as *mut std::ffi::c_void;
    unsafe {
        dispatch_async_f(main_queue(), ptr, trampoline);
    }
}

fn dispatch_to_main_after<F: FnOnce() + Send + 'static>(delay_ms: i64, f: F) {
    let boxed: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(f));
    let ptr = Box::into_raw(boxed) as *mut std::ffi::c_void;
    unsafe {
        let when = dispatch_time(DISPATCH_TIME_NOW, delay_ms * NSEC_PER_SEC / 1000);
        dispatch_after_f(when, main_queue(), ptr, trampoline);
    }
}

unsafe extern "C" fn trampoline(ptr: *mut std::ffi::c_void) {
    let boxed: Box<Box<dyn FnOnce() + Send>> = Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>);
    (*boxed)();
}

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    /// Whether the commands-unavailable sign shows beside the state.
    commands_unavailable: Arc<AtomicBool>,
    windows: Arc<Mutex<Vec<Arc<WindowSet>>>>,
    pulse_generation: Arc<AtomicU64>,
    level_register: Arc<AtomicU32>,
    live_transcript: LiveTranscriptText,
    #[allow(dead_code)]
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
        let fallback_rect = objc2_foundation::NSRect {
            origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
            size: objc2_foundation::NSSize {
                width: 1440.0,
                height: 900.0,
            },
        };

        let screens = NSScreen::screens(mtm);
        let initial_windows: Vec<Arc<WindowSet>> = if screens.is_empty() {
            vec![WindowSet::build(mtm, fallback_rect)]
        } else {
            (0..screens.count())
                .map(|i| {
                    let screen = screens.objectAtIndex(i);
                    WindowSet::build(mtm, screen.visibleFrame())
                })
                .collect()
        };

        let initial_topology = collect_topology(mtm);

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let state = Arc::new(Mutex::new(OverlayState::Hidden));
        let windows = Arc::new(Mutex::new(initial_windows));
        let pulse_generation = Arc::new(AtomicU64::new(0));
        let last_topology = Arc::new(Mutex::new(initial_topology));
        let live_transcript = LiveTranscriptText::default();

        let commands_unavailable = Arc::new(AtomicBool::new(false));
        let redraw = Redraw {
            windows: Arc::clone(&windows),
            level_register: Arc::clone(&level_register),
            live_transcript: live_transcript.clone(),
            timer: NoticeTimer {
                duration_ms: notice_duration_ms,
                pulse_generation: Arc::clone(&pulse_generation),
                state: Arc::clone(&state),
                commands_unavailable: Arc::clone(&commands_unavailable),
            },
        };
        let last_topology_for_watcher = Arc::clone(&last_topology);

        let handler: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(move || {
            let redraw = redraw.clone();
            let last_topology = Arc::clone(&last_topology_for_watcher);

            dispatch_to_main(move || {
                let Some(mtm) = MainThreadMarker::new() else {
                    return;
                };

                let generation = redraw.timer.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;

                do_rebuild(mtm, &redraw.windows, &last_topology);

                let current_state = *redraw.timer.state.lock().unwrap();
                redraw.draw(current_state, generation);
            });
        });

        let watcher = ScreenWatcher::install(handler);

        Self {
            state,
            commands_unavailable,
            windows,
            pulse_generation,
            level_register,
            live_transcript,
            last_topology,
            notice_duration_ms,
            _watcher: watcher,
        }
    }

    #[allow(dead_code)]
    pub fn rebuild_window_set(&self, mtm: MainThreadMarker) {
        rebuild_window_set_inner(mtm, &self.last_topology, &self.redraw());
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
            level_register: Arc::clone(&self.level_register),
            live_transcript: self.live_transcript.clone(),
            timer: self.notice_timer(),
        }
    }

    pub fn generation_count(&self) -> u64 {
        self.pulse_generation
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn first_eq_label_value(&self) -> String {
        self.windows
            .lock()
            .unwrap()
            .first()
            .map(|ws| ws.eq_label.stringValue().to_string())
            .unwrap_or_default()
    }

    /// The sign the pill of the first screen shows.
    pub fn first_sign_value(&self) -> String {
        self.windows
            .lock()
            .unwrap()
            .first()
            .map(|ws| ws.label.stringValue().to_string())
            .unwrap_or_default()
    }

    /// The text the window under the pill of the first screen shows; `None` while it is hidden.
    pub fn first_text_window_value(&self) -> Option<String> {
        let windows = self.windows.lock().unwrap();
        let transcript = &windows.first()?.transcript;
        transcript
            .window
            .isVisible()
            .then(|| transcript.label.stringValue().to_string())
    }

    /// Whether the pill of the first screen is on screen.
    pub fn is_pill_visible(&self) -> bool {
        self.windows
            .lock()
            .unwrap()
            .first()
            .is_some_and(|ws| ws.window.isVisible())
    }

    /// Whether any overlay window took the keyboard focus.
    pub fn has_key_window(&self) -> bool {
        self.windows
            .lock()
            .unwrap()
            .iter()
            .any(|ws| ws.window.isKeyWindow() || ws.transcript.window.isKeyWindow())
    }
}

#[allow(dead_code)]
fn windows_are_usable(windows: &[Arc<WindowSet>]) -> bool {
    if windows.is_empty() {
        return false;
    }
    let ws = &windows[0];
    ws.window.windowNumber() > 0 || ws.window.screen().is_some()
}

fn do_rebuild(
    mtm: MainThreadMarker,
    windows: &Arc<Mutex<Vec<Arc<WindowSet>>>>,
    last_topology: &Arc<Mutex<ScreenTopology>>,
) {
    let new_topology = collect_topology(mtm);

    let fallback_rect = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
        size: objc2_foundation::NSSize {
            width: 1440.0,
            height: 900.0,
        },
    };

    let screens = NSScreen::screens(mtm);
    let new_windows: Vec<Arc<WindowSet>> = if screens.is_empty() {
        vec![WindowSet::build(mtm, fallback_rect)]
    } else {
        (0..screens.count())
            .map(|i| WindowSet::build(mtm, screens.objectAtIndex(i).visibleFrame()))
            .collect()
    };

    {
        let mut guard = windows.lock().unwrap();
        let old_windows = std::mem::replace(&mut *guard, new_windows);
        drop(guard);
        drop(old_windows);
    }

    *last_topology.lock().unwrap() = new_topology;
}

fn collect_topology(mtm: MainThreadMarker) -> ScreenTopology {
    let screens = NSScreen::screens(mtm);
    let frames: Vec<objc2_foundation::NSRect> = (0..screens.count())
        .map(|i| screens.objectAtIndex(i).visibleFrame())
        .collect();
    ScreenTopology { frames }
}

/// Rebuilds the windows when the screens changed and redraws the state the timer's state holds.
fn rebuild_window_set_inner(
    mtm: MainThreadMarker,
    last_topology: &Arc<Mutex<ScreenTopology>>,
    redraw: &Redraw,
) {
    let new_topology = collect_topology(mtm);

    {
        let current = last_topology.lock().unwrap();
        if diff_screens(&current, &new_topology) == ScreenChange::Identical {
            return;
        }
    }

    let generation = redraw.timer.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;

    do_rebuild(mtm, &redraw.windows, last_topology);

    let current_state = *redraw.timer.state.lock().unwrap();
    redraw.draw(current_state, generation);
}

/// What draws a state of the overlay on the windows of the moment, with the
/// commands-unavailable sign while it is on.
#[derive(Clone)]
struct Redraw {
    windows: Arc<Mutex<Vec<Arc<WindowSet>>>>,
    level_register: Arc<AtomicU32>,
    live_transcript: LiveTranscriptText,
    timer: NoticeTimer,
}

impl Redraw {
    fn draw(&self, state: OverlayState, generation: u64) {
        let commands_unavailable = self.timer.commands_unavailable.load(Ordering::SeqCst);
        let pulse_gen = Arc::clone(&self.timer.pulse_generation);
        let ws_vec = self.windows.lock().unwrap().clone();
        match state {
            OverlayState::Recording => {
                let level_register = Arc::clone(&self.level_register);
                let shown = self.live_transcript.shown(state);
                apply_recording_state(
                    ws_vec.clone(),
                    generation,
                    pulse_gen,
                    level_register,
                    commands_unavailable,
                );
                apply_live_transcript(&ws_vec, shown);
            }
            OverlayState::RecognizingSpeech | OverlayState::RefiningText => {
                apply_processing_state(ws_vec, state, generation, pulse_gen, commands_unavailable);
            }
            OverlayState::WaitingForFocus => {
                apply_waiting_for_focus_state(ws_vec, generation, pulse_gen, commands_unavailable);
            }
            OverlayState::FailureNotice(notice) => {
                show_notice(&self.windows, &notice, generation, self.timer.clone());
            }
            OverlayState::Hidden => apply_hidden_state(ws_vec, commands_unavailable),
        }
    }
}

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = Arc::clone(&self.windows);
        let last_topology = Arc::clone(&self.last_topology);
        let level_register = Arc::clone(&self.level_register);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *self.state.lock().unwrap() = OverlayState::Recording;
        self.live_transcript.clear();
        log::debug!("overlay: recording requested");

        dispatch_to_main(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                do_rebuild(mtm, &windows, &last_topology);

                let ws_vec = windows.lock().unwrap().clone();
                log::debug!("overlay: recording shown on {} screen(s)", ws_vec.len());
                let commands_unavailable = commands_unavailable.load(Ordering::SeqCst);
                apply_recording_state(
                    ws_vec,
                    generation,
                    pulse_gen,
                    level_register,
                    commands_unavailable,
                );
            }
        });
    }

    fn show_recognizing_speech(&self) {
        self.show_processing(OverlayState::RecognizingSpeech);
    }

    fn show_refining_text(&self) {
        self.show_processing(OverlayState::RefiningText);
    }

    fn show_waiting_for_focus(&self) {
        self.show_waiting();
    }

    fn hide(&self) {
        self.hide_overlay();
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let windows = Arc::clone(&self.windows);
        let last_topology = Arc::clone(&self.last_topology);
        let timer = self.notice_timer();
        let notice = *notice;

        *self.state.lock().unwrap() = OverlayState::FailureNotice(notice);
        log::warn!("overlay: failure notice: {}", notice.message());

        dispatch_to_main(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                do_rebuild(mtm, &windows, &last_topology);
                show_notice(&windows, &notice, generation, timer);
            }
        });
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        let Some(sign) = command_sign(command) else {
            return;
        };
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = Arc::clone(&self.windows);
        let last_topology = Arc::clone(&self.last_topology);
        let level_register = Arc::clone(&self.level_register);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);
        // The state stays what the command left, so a screen change redraws that state.
        let after = state_after_sign(*self.state.lock().unwrap());
        log::debug!("overlay: command sign {} requested", sign.1);

        dispatch_to_main(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            do_rebuild(mtm, &windows, &last_topology);
            apply_command_sign(&windows.lock().unwrap(), sign);
            dispatch_to_main_after(COMMAND_SIGN_MS, move || {
                // WHY: a later signal took over the overlay; its own drawing stands
                if pulse_gen.load(Ordering::SeqCst) != generation {
                    return;
                }
                let ws_vec = windows.lock().unwrap().clone();
                let commands_unavailable = commands_unavailable.load(Ordering::SeqCst);
                match after {
                    OverlayState::Recording => apply_recording_state(
                        ws_vec,
                        generation,
                        pulse_gen,
                        level_register,
                        commands_unavailable,
                    ),
                    _ => apply_hidden_state(ws_vec, commands_unavailable),
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
    windows: Arc<Mutex<Vec<Arc<WindowSet>>>>,
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
        ws.transcript.show(shown.as_deref());
    }
}

impl MacosVisualIndicator {
    fn show_processing(&self, stage: OverlayState) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = Arc::clone(&self.windows);
        let last_topology = Arc::clone(&self.last_topology);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *self.state.lock().unwrap() = stage;
        log::debug!("overlay: {stage:?} requested");

        dispatch_to_main(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                do_rebuild(mtm, &windows, &last_topology);

                let ws_vec = windows.lock().unwrap().clone();
                log::debug!("overlay: {stage:?} shown on {} screen(s)", ws_vec.len());
                let commands_unavailable = commands_unavailable.load(Ordering::SeqCst);
                apply_processing_state(ws_vec, stage, generation, pulse_gen, commands_unavailable);
            }
        });
    }

    fn show_waiting(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = Arc::clone(&self.windows);
        let last_topology = Arc::clone(&self.last_topology);
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *self.state.lock().unwrap() = OverlayState::WaitingForFocus;

        dispatch_to_main(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                do_rebuild(mtm, &windows, &last_topology);

                let ws_vec = windows.lock().unwrap().clone();
                let commands_unavailable = commands_unavailable.load(Ordering::SeqCst);
                apply_waiting_for_focus_state(ws_vec, generation, pulse_gen, commands_unavailable);
            }
        });
    }

    fn hide_overlay(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let windows = self.windows.lock().unwrap().clone();
        let commands_unavailable = Arc::clone(&self.commands_unavailable);

        *self.state.lock().unwrap() = OverlayState::Hidden;
        log::debug!("overlay: hide requested");

        dispatch_to_main(move || {
            log::debug!("overlay: hidden");
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
            let generation = redraw.timer.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
            redraw.draw(state, generation);
        });
    }
}

impl AudioLevelSource for MacosVisualIndicator {
    fn read_level(&self) -> AudioLevel {
        AudioLevel::from_bits(self.level_register.load(Ordering::Relaxed))
    }
}

unsafe impl Send for MacosVisualIndicator {}

fn apply_recording_state(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    level_register: Arc<AtomicU32>,
    commands_unavailable: bool,
) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let sign = pill_sign(OverlayState::Recording, commands_unavailable).unwrap_or_default();
    let text = NSString::from_str(&sign);
    for ws in &windows {
        ws.label.setStringValue(&text);
        ws.set_width(pill_width(commands_unavailable));
        ws.label.setTextColor(Some(&NSColor::redColor()));
        ws.label.setHidden(false);
        ws.transcript.show(None);
        ws.eq_label.setHidden(false);
        ws.spinner.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows.clone(), generation, Arc::clone(&pulse_gen));
    schedule_pulse(windows.clone(), generation, Arc::clone(&pulse_gen), false);
    schedule_equalizer_tick(windows, generation, pulse_gen, level_register);
}

fn schedule_pulse(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    bright: bool,
) {
    let pulse_ms: i64 = 600;
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, pulse_ms * NSEC_PER_SEC / 1000) };
    let data: Box<PulseData> = Box::new((windows, generation, pulse_gen, bright));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, pulse_trampoline);
    }
}

unsafe extern "C" fn pulse_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<PulseData> = Box::from_raw(ptr as *mut PulseData);
    let (windows, generation, pulse_gen, bright) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    let alpha: f64 = if bright { 1.0 } else { 0.35 };
    for ws in &windows {
        ws.label.setAlphaValue(alpha);
    }

    schedule_pulse(windows, generation, pulse_gen, !bright);
}

/// The icon of the processing stage on the left, the spinner turning to its right.
fn apply_processing_state(
    windows: Vec<Arc<WindowSet>>,
    stage: OverlayState,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    commands_unavailable: bool,
) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let icon = NSString::from_str(&pill_sign(stage, commands_unavailable).unwrap_or_default());
    for ws in &windows {
        ws.label.setStringValue(&icon);
        ws.set_width(pill_width(commands_unavailable));
        ws.label.setTextColor(Some(&NSColor::labelColor()));
        ws.label.setAlphaValue(1.0);
        ws.label.setHidden(false);
        ws.eq_label.setHidden(true);
        ws.transcript.show(None);
        ws.spinner.setHidden(false);
        unsafe { ws.spinner.startAnimation(None) };
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows, generation, pulse_gen);
}

/// Pulsing orange label: the text is held back until a text input gets focus.
fn apply_waiting_for_focus_state(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    commands_unavailable: bool,
) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let sign = pill_sign(OverlayState::WaitingForFocus, commands_unavailable).unwrap_or_default();
    let text = NSString::from_str(&sign);
    for ws in &windows {
        ws.label.setStringValue(&text);
        ws.set_width(pill_width(commands_unavailable));
        ws.label.setTextColor(Some(&NSColor::orangeColor()));
        ws.label.setHidden(false);
        ws.transcript.show(None);
        ws.eq_label.setHidden(true);
        ws.spinner.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows.clone(), generation, Arc::clone(&pulse_gen));
    schedule_pulse(windows, generation, pulse_gen, false);
}

fn schedule_heartbeat(windows: Vec<Arc<WindowSet>>, generation: u64, pulse_gen: Arc<AtomicU64>) {
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, 200 * NSEC_PER_SEC / 1000) };
    let data: Box<HeartbeatData> = Box::new((windows, generation, pulse_gen));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, heartbeat_trampoline);
    }
}

unsafe extern "C" fn heartbeat_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<HeartbeatData> = Box::from_raw(ptr as *mut HeartbeatData);
    let (windows, generation, pulse_gen) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    use objc2_app_kit::NSWindowCollectionBehavior;
    for ws in &windows {
        ws.window.setLevel(3);
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows, generation, pulse_gen);
}

/// The command sign in place of the state's sign and level, steady, with no spinner; a live
/// transcript under the pill stays as it is.
fn apply_command_sign(windows: &[Arc<WindowSet>], (icon, name): CommandSign) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let (icon, name) = (NSString::from_str(icon), NSString::from_str(name));
    for ws in windows {
        ws.label.setStringValue(&icon);
        ws.label.setTextColor(Some(&NSColor::labelColor()));
        ws.label.setAlphaValue(1.0);
        ws.label.setHidden(false);
        ws.eq_label.setStringValue(&name);
        ws.eq_label.setHidden(false);
        ws.spinner.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

/// Hides the pill, or leaves the commands-unavailable sign alone in it while it is on.
fn apply_hidden_state(windows: Vec<Arc<WindowSet>>, commands_unavailable: bool) {
    if commands_unavailable {
        apply_commands_unavailable_alone(&windows);
        return;
    }
    for ws in &windows {
        ws.eq_label.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.window.orderOut(None);
        ws.transcript.show(None);
    }
}

/// The commands-unavailable sign alone in a pill just wide enough for it, steady and grey.
fn apply_commands_unavailable_alone(windows: &[Arc<WindowSet>]) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let sign = pill_sign(OverlayState::Hidden, true).unwrap_or_default();
    let text = NSString::from_str(&sign);
    for ws in windows {
        ws.label.setStringValue(&text);
        ws.set_width(PillWidth::SignAlone);
        ws.label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        ws.label.setAlphaValue(1.0);
        ws.label.setHidden(false);
        ws.eq_label.setHidden(true);
        ws.spinner.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.transcript.show(None);
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

fn schedule_equalizer_tick(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    level_register: Arc<AtomicU32>,
) {
    let tick_ms: i64 = 80;
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, tick_ms * NSEC_PER_SEC / 1000) };
    let data: Box<EqualizerData> = Box::new((windows, generation, pulse_gen, level_register));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, equalizer_trampoline);
    }
}

unsafe extern "C" fn equalizer_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<EqualizerData> = Box::from_raw(ptr as *mut EqualizerData);
    let (windows, generation, pulse_gen, level_register) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    use objc2_foundation::NSString;

    let bits = level_register.load(Ordering::Relaxed);
    let level = AudioLevel::from_bits(bits);

    let pills = level_to_pills(level.value());
    let pills_str = NSString::from_str(&pills);

    let alpha: f64 = if level.value() >= 0.01 { 1.0 } else { 0.55 };

    for ws in &windows {
        ws.eq_label.setStringValue(&pills_str);
        ws.eq_label.setAlphaValue(alpha);
    }

    schedule_equalizer_tick(windows, generation, pulse_gen, level_register);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_each_command_without_a_visible_effect_in_its_sign() {
        // Given
        let signed = [
            VoiceCommand::PressEnter,
            VoiceCommand::ClearAllText,
            VoiceCommand::PasteText,
            VoiceCommand::RestartDictation,
        ];
        // When
        let signs: Vec<_> = signed.into_iter().map(command_sign).collect();
        // Then
        assert_eq!(
            signs,
            [
                Some(("⏎", "ENTER")),
                Some(("⌫", "CLEAR")),
                Some(("⎘", "PASTE")),
                Some(("↻", "RESTART"))
            ]
        );
    }

    #[test]
    fn should_give_no_sign_to_commands_the_overlay_already_shows() {
        // Given
        let shown = [
            VoiceCommand::StartDictation,
            VoiceCommand::StopDictation,
            VoiceCommand::CancelDictation,
        ];
        // When / Then
        for command in shown {
            assert_eq!(command_sign(command), None, "{command:?}");
        }
    }

    #[test]
    fn should_return_to_the_recording_after_a_sign_shown_during_it() {
        assert_eq!(
            state_after_sign(OverlayState::Recording),
            OverlayState::Recording
        );
    }

    #[test]
    fn should_hide_after_a_sign_shown_outside_a_recording() {
        // Given
        let others = [
            OverlayState::Hidden,
            OverlayState::RecognizingSpeech,
            OverlayState::RefiningText,
            OverlayState::WaitingForFocus,
        ];
        // When / Then
        for state in others {
            assert_eq!(state_after_sign(state), OverlayState::Hidden, "{state:?}");
        }
    }

    #[test]
    fn should_give_each_processing_stage_an_icon_of_its_own() {
        let stages = [OverlayState::RecognizingSpeech, OverlayState::RefiningText];
        let icons: Vec<&str> = stages.iter().filter_map(|s| stage_icon(*s)).collect();
        assert_eq!(icons.len(), 2, "every processing stage has an icon");
        assert_ne!(icons[0], icons[1]);
        for icon in icons {
            assert_ne!(icon, RECORDING_SIGN);
            assert_ne!(icon, WAITING_FOR_FOCUS_SIGN);
        }
    }

    #[test]
    fn should_show_headphones_for_the_speech_recognition_stage() {
        assert_eq!(
            stage_icon(OverlayState::RecognizingSpeech),
            Some("\u{1F3A7}")
        );
    }

    #[test]
    fn should_show_sparkles_for_the_text_refinement_stage() {
        assert_eq!(stage_icon(OverlayState::RefiningText), Some("\u{2728}"));
    }

    #[test]
    fn should_show_the_commands_unavailable_sign_after_the_recording_sign() {
        assert_eq!(
            pill_sign(OverlayState::Recording, true).as_deref(),
            Some("◉ REC ⊘ CMD")
        );
    }

    #[test]
    fn should_show_the_commands_unavailable_sign_after_a_stage_icon() {
        // Given
        let stages = [OverlayState::RecognizingSpeech, OverlayState::RefiningText];
        // When / Then
        for stage in stages {
            let expected = format!("{} ⊘ CMD", stage_icon(stage).unwrap());
            assert_eq!(pill_sign(stage, true), Some(expected), "{stage:?}");
        }
    }

    #[test]
    fn should_show_the_commands_unavailable_sign_alone_while_hidden() {
        assert_eq!(
            pill_sign(OverlayState::Hidden, true).as_deref(),
            Some("⊘ CMD")
        );
    }

    #[test]
    fn should_show_each_state_as_before_while_commands_are_available() {
        // Given
        let expected = [
            (OverlayState::Hidden, None),
            (OverlayState::Recording, Some(RECORDING_SIGN)),
            (OverlayState::RecognizingSpeech, Some("\u{1F3A7}")),
            (OverlayState::RefiningText, Some("\u{2728}")),
            (OverlayState::WaitingForFocus, Some(WAITING_FOR_FOCUS_SIGN)),
        ];
        // When / Then
        for (state, sign) in expected {
            assert_eq!(pill_sign(state, false).as_deref(), sign, "{state:?}");
        }
    }

    #[test]
    fn should_read_level_from_register_via_audio_level_source() {
        let reg = Arc::new(AtomicU32::new(0.5f32.to_bits()));
        let level = AudioLevel::from_bits(reg.load(Ordering::Relaxed));
        assert!((level.value() - 0.5).abs() < 1e-6);
    }
}
