mod screen_watcher;
mod topology;
mod window_set;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSScreen};

use crate::core::application::{AudioLevelSource, VisualIndicator};
use crate::core::domain::{level_to_block, AudioLevel};

use screen_watcher::ScreenWatcher;
use topology::{diff_screens, ScreenChange, ScreenTopology};
use window_set::WindowSet;

type PulseData = (Vec<Arc<WindowSet>>, u64, Arc<AtomicU64>, bool);
type HeartbeatData = (Vec<Arc<WindowSet>>, u64, Arc<AtomicU64>);
type EqualizerData = (
    Vec<Arc<WindowSet>>,
    u64,
    Arc<AtomicU64>,
    Arc<AtomicU32>,
    [f32; 10],
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    Processing,
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

unsafe extern "C" fn trampoline(ptr: *mut std::ffi::c_void) {
    let boxed: Box<Box<dyn FnOnce() + Send>> = Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>);
    (*boxed)();
}

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    windows: Arc<Mutex<Vec<Arc<WindowSet>>>>,
    pulse_generation: Arc<AtomicU64>,
    level_register: Arc<AtomicU32>,
    #[allow(dead_code)]
    last_topology: Arc<Mutex<ScreenTopology>>,
    _watcher: ScreenWatcher,
}

impl MacosVisualIndicator {
    pub fn initialize(mtm: MainThreadMarker, level_register: Arc<AtomicU32>) -> Self {
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

        let state_for_watcher = Arc::clone(&state);
        let windows_for_watcher = Arc::clone(&windows);
        let pulse_generation_for_watcher = Arc::clone(&pulse_generation);
        let last_topology_for_watcher = Arc::clone(&last_topology);
        let level_register_for_watcher = Arc::clone(&level_register);

        let handler: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(move || {
            let state = Arc::clone(&state_for_watcher);
            let windows = Arc::clone(&windows_for_watcher);
            let pulse_generation = Arc::clone(&pulse_generation_for_watcher);
            let last_topology = Arc::clone(&last_topology_for_watcher);
            let level_register = Arc::clone(&level_register_for_watcher);

            dispatch_to_main(move || {
                let Some(mtm) = MainThreadMarker::new() else {
                    return;
                };
                rebuild_window_set_inner(
                    mtm,
                    &state,
                    &windows,
                    &pulse_generation,
                    &last_topology,
                    &level_register,
                );
            });
        });

        let watcher = ScreenWatcher::install(handler);

        Self {
            state,
            windows,
            pulse_generation,
            level_register,
            last_topology,
            _watcher: watcher,
        }
    }

    #[allow(dead_code)]
    pub fn rebuild_window_set(&self, mtm: MainThreadMarker) {
        rebuild_window_set_inner(
            mtm,
            &self.state,
            &self.windows,
            &self.pulse_generation,
            &self.last_topology,
            &self.level_register,
        );
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
}

fn collect_topology(mtm: MainThreadMarker) -> ScreenTopology {
    let screens = NSScreen::screens(mtm);
    let frames: Vec<objc2_foundation::NSRect> = (0..screens.count())
        .map(|i| screens.objectAtIndex(i).visibleFrame())
        .collect();
    ScreenTopology { frames }
}

fn rebuild_window_set_inner(
    mtm: MainThreadMarker,
    state: &Arc<Mutex<OverlayState>>,
    windows: &Arc<Mutex<Vec<Arc<WindowSet>>>>,
    pulse_generation: &Arc<AtomicU64>,
    last_topology: &Arc<Mutex<ScreenTopology>>,
    level_register: &Arc<AtomicU32>,
) {
    let new_topology = collect_topology(mtm);

    {
        let current = last_topology.lock().unwrap();
        if diff_screens(&current, &new_topology) == ScreenChange::Identical {
            return;
        }
    }

    let generation = pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;

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

    let current_state = *state.lock().unwrap();
    match current_state {
        OverlayState::Recording => {
            let ws_vec = windows.lock().unwrap().clone();
            let pg = Arc::clone(pulse_generation);
            let lr = Arc::clone(level_register);
            apply_recording_state(ws_vec, generation, pg, lr);
        }
        OverlayState::Processing => {
            let ws_vec = windows.lock().unwrap().clone();
            let pg = Arc::clone(pulse_generation);
            apply_processing_state(ws_vec, generation, pg);
        }
        OverlayState::Hidden => {}
    }
}

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = self.windows.lock().unwrap().clone();
        let level_register = Arc::clone(&self.level_register);

        *self.state.lock().unwrap() = OverlayState::Recording;

        dispatch_to_main(move || {
            apply_recording_state(windows, generation, pulse_gen, level_register);
        });
    }

    fn show_processing(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = self.windows.lock().unwrap().clone();

        *self.state.lock().unwrap() = OverlayState::Processing;

        dispatch_to_main(move || {
            apply_processing_state(windows, generation, pulse_gen);
        });
    }

    fn hide(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let windows = self.windows.lock().unwrap().clone();

        *self.state.lock().unwrap() = OverlayState::Hidden;

        dispatch_to_main(move || {
            apply_hidden_state(windows);
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
) {
    use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
    use objc2_foundation::NSString;

    let text = NSString::from_str("◉ REC");
    for ws in &windows {
        ws.label.setStringValue(&text);
        ws.label.setTextColor(Some(&NSColor::redColor()));
        ws.label.setHidden(false);
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
    schedule_equalizer_tick(windows, generation, pulse_gen, level_register, [0.0f32; 10]);
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

fn apply_processing_state(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
) {
    use objc2_app_kit::NSWindowCollectionBehavior;

    for ws in &windows {
        ws.label.setHidden(true);
        ws.eq_label.setHidden(true);
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

fn apply_hidden_state(windows: Vec<Arc<WindowSet>>) {
    for ws in &windows {
        ws.eq_label.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.window.orderOut(None);
    }
}

fn schedule_equalizer_tick(
    windows: Vec<Arc<WindowSet>>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    level_register: Arc<AtomicU32>,
    eq_history: [f32; 10],
) {
    let tick_ms: i64 = 80;
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, tick_ms * NSEC_PER_SEC / 1000) };
    let data: Box<EqualizerData> =
        Box::new((windows, generation, pulse_gen, level_register, eq_history));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, equalizer_trampoline);
    }
}

unsafe extern "C" fn equalizer_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<EqualizerData> = Box::from_raw(ptr as *mut EqualizerData);
    let (windows, generation, pulse_gen, level_register, eq_history) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    use objc2_foundation::NSString;

    let bits = level_register.load(Ordering::Relaxed);
    let level = AudioLevel::from_bits(bits);

    let mut new_history = [0_f32; 10];
    new_history[..9].copy_from_slice(&eq_history[1..]);
    new_history[9] = level.value();

    let blocks: String = new_history.iter().map(|&v| level_to_block(v)).collect();
    let blocks_str = NSString::from_str(&blocks);

    let any_active = new_history.iter().any(|&v| v >= 0.01);
    let alpha: f64 = if any_active { 1.0 } else { 0.55 };

    for ws in &windows {
        ws.eq_label.setStringValue(&blocks_str);
        ws.eq_label.setAlphaValue(alpha);
    }

    schedule_equalizer_tick(windows, generation, pulse_gen, level_register, new_history);
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
