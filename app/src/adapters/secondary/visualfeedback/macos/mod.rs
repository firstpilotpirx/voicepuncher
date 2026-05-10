use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSControlSize,
    NSFont, NSProgressIndicator, NSProgressIndicatorStyle, NSScreen, NSTextAlignment, NSTextField,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::NSString;

use crate::core::application::VisualIndicator;

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

#[derive(Clone)]
struct WindowSet {
    window_ptr: usize,
    label_ptr: usize,
    spinner_ptr: usize,
}

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    windows: Vec<WindowSet>,
    pulse_generation: Arc<AtomicU64>,
}

fn build_window_set(
    mtm: MainThreadMarker,
    screen_visible_frame: objc2_foundation::NSRect,
) -> WindowSet {
    let window_width: f64 = 140.0;
    let window_height: f64 = 55.0;

    let x = screen_visible_frame.origin.x
        + (screen_visible_frame.size.width - window_width) / 2.0;
    let y = screen_visible_frame.origin.y + screen_visible_frame.size.height
        - window_height
        - 4.0;

    let frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint { x, y },
        size: objc2_foundation::NSSize {
            width: window_width,
            height: window_height,
        },
    };

    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };

    window.setLevel(3);
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );

    let content_frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
        size: objc2_foundation::NSSize {
            width: window_width,
            height: window_height,
        },
    };
    let effect_view = NSVisualEffectView::initWithFrame(
        mtm.alloc::<NSVisualEffectView>(),
        content_frame,
    );
    effect_view.setMaterial(NSVisualEffectMaterial::HUDWindow);
    effect_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    effect_view.setState(NSVisualEffectState::Active);
    effect_view.setWantsLayer(true);
    if let Some(layer) = effect_view.layer() {
        layer.setCornerRadius(28.0);
        layer.setMasksToBounds(true);
        let dark_fill = NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.28);
        layer.setBackgroundColor(Some(&dark_fill.CGColor()));
        let border = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.25);
        layer.setBorderColor(Some(&border.CGColor()));
        layer.setBorderWidth(0.5);
    }
    window.setContentView(Some(&*effect_view));

    let font = NSFont::boldSystemFontOfSize(28.0);
    let label_height = (font.ascender() - font.descender()).ceil() + 4.0;
    let label_y = (window_height / 2.0 - font.capHeight() / 2.0 - (label_height - font.ascender())).floor();
    let label_frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint {
            x: 12.0,
            y: label_y,
        },
        size: objc2_foundation::NSSize {
            width: window_width - 24.0,
            height: label_height,
        },
    };

    let label = NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), label_frame);
    label.setBezeled(false);
    label.setEditable(false);
    label.setDrawsBackground(false);
    label.setAlignment(NSTextAlignment::Center);
    label.setFont(Some(&font));
    label.setTextColor(Some(&NSColor::whiteColor()));
    let empty = NSString::from_str("");
    label.setStringValue(&empty);

    effect_view.addSubview(&label);

    let spinner_size: f64 = 32.0;
    let spinner_x = (window_width - spinner_size) / 2.0;
    let spinner_frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint {
            x: spinner_x,
            y: (window_height - spinner_size) / 2.0,
        },
        size: objc2_foundation::NSSize {
            width: spinner_size,
            height: spinner_size,
        },
    };

    let spinner =
        NSProgressIndicator::initWithFrame(mtm.alloc::<NSProgressIndicator>(), spinner_frame);
    spinner.setStyle(NSProgressIndicatorStyle::Spinning);
    spinner.setIndeterminate(true);
    spinner.setControlSize(NSControlSize::Regular);
    spinner.setHidden(true);

    effect_view.addSubview(&spinner);

    let window_ptr = Retained::as_ptr(&window) as usize;
    let label_ptr = Retained::as_ptr(&label) as usize;
    let spinner_ptr = Retained::as_ptr(&spinner) as usize;

    let _ = Retained::into_raw(window);
    let _ = Retained::into_raw(label);
    let _ = Retained::into_raw(spinner);

    WindowSet {
        window_ptr,
        label_ptr,
        spinner_ptr,
    }
}

impl MacosVisualIndicator {
    pub fn initialize(mtm: MainThreadMarker) -> Self {
        let fallback_rect = objc2_foundation::NSRect {
            origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
            size: objc2_foundation::NSSize {
                width: 1440.0,
                height: 900.0,
            },
        };

        let screens = NSScreen::screens(mtm);
        let windows: Vec<WindowSet> = if screens.is_empty() {
            vec![build_window_set(mtm, fallback_rect)]
        } else {
            (0..screens.count())
                .map(|i| {
                    let screen = screens.objectAtIndex(i);
                    build_window_set(mtm, screen.visibleFrame())
                })
                .collect()
        };

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        Self {
            state: Arc::new(Mutex::new(OverlayState::Hidden)),
            windows,
            pulse_generation: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = self.windows.clone();

        *self.state.lock().unwrap() = OverlayState::Recording;

        dispatch_to_main(move || {
            apply_recording_state(windows, generation, pulse_gen);
        });
    }

    fn show_processing(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let windows = self.windows.clone();

        *self.state.lock().unwrap() = OverlayState::Processing;

        dispatch_to_main(move || {
            apply_processing_state(windows, generation, pulse_gen);
        });
    }

    fn hide(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let windows = self.windows.clone();

        *self.state.lock().unwrap() = OverlayState::Hidden;

        dispatch_to_main(move || {
            apply_hidden_state(windows);
        });
    }
}

unsafe impl Send for MacosVisualIndicator {}

fn apply_recording_state(
    windows: Vec<WindowSet>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
) {
    let text = NSString::from_str("◉ REC");
    for ws in &windows {
        let window = unsafe { (ws.window_ptr as *const NSWindow).as_ref().unwrap() };
        let label = unsafe { (ws.label_ptr as *const NSTextField).as_ref().unwrap() };
        let spinner = unsafe {
            (ws.spinner_ptr as *const NSProgressIndicator)
                .as_ref()
                .unwrap()
        };

        label.setStringValue(&text);
        label.setTextColor(Some(&NSColor::redColor()));
        label.setHidden(false);
        spinner.setHidden(true);
        unsafe { spinner.stopAnimation(None) };
        window.orderFrontRegardless();
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows.clone(), generation, Arc::clone(&pulse_gen));
    schedule_pulse(windows, generation, pulse_gen, false);
}

fn schedule_pulse(
    windows: Vec<WindowSet>,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    bright: bool,
) {
    let pulse_ms: i64 = 600;
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, pulse_ms * NSEC_PER_SEC / 1000) };
    let data: Box<(Vec<WindowSet>, u64, Arc<AtomicU64>, bool)> =
        Box::new((windows, generation, pulse_gen, bright));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, pulse_trampoline);
    }
}

unsafe extern "C" fn pulse_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<(Vec<WindowSet>, u64, Arc<AtomicU64>, bool)> =
        Box::from_raw(ptr as *mut (Vec<WindowSet>, u64, Arc<AtomicU64>, bool));
    let (windows, generation, pulse_gen, bright) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    let alpha: f64 = if bright { 1.0 } else { 0.35 };
    for ws in &windows {
        let label = (ws.label_ptr as *const NSTextField).as_ref().unwrap();
        label.setAlphaValue(alpha);
    }

    schedule_pulse(windows, generation, pulse_gen, !bright);
}

fn apply_processing_state(windows: Vec<WindowSet>, generation: u64, pulse_gen: Arc<AtomicU64>) {
    for ws in &windows {
        let window = unsafe { (ws.window_ptr as *const NSWindow).as_ref().unwrap() };
        let label = unsafe { (ws.label_ptr as *const NSTextField).as_ref().unwrap() };
        let spinner = unsafe {
            (ws.spinner_ptr as *const NSProgressIndicator)
                .as_ref()
                .unwrap()
        };

        label.setHidden(true);
        spinner.setHidden(false);
        unsafe { spinner.startAnimation(None) };
        window.orderFrontRegardless();
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows, generation, pulse_gen);
}

fn schedule_heartbeat(windows: Vec<WindowSet>, generation: u64, pulse_gen: Arc<AtomicU64>) {
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, 200 * NSEC_PER_SEC / 1000) };
    let data: Box<(Vec<WindowSet>, u64, Arc<AtomicU64>)> =
        Box::new((windows, generation, pulse_gen));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, heartbeat_trampoline);
    }
}

unsafe extern "C" fn heartbeat_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<(Vec<WindowSet>, u64, Arc<AtomicU64>)> =
        Box::from_raw(ptr as *mut (Vec<WindowSet>, u64, Arc<AtomicU64>));
    let (windows, generation, pulse_gen) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    for ws in &windows {
        let window = (ws.window_ptr as *const NSWindow).as_ref().unwrap();
        window.setLevel(3);
        window.orderFrontRegardless();
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    schedule_heartbeat(windows, generation, pulse_gen);
}

fn apply_hidden_state(windows: Vec<WindowSet>) {
    for ws in &windows {
        let window = unsafe { (ws.window_ptr as *const NSWindow).as_ref().unwrap() };
        let spinner = unsafe {
            (ws.spinner_ptr as *const NSProgressIndicator)
                .as_ref()
                .unwrap()
        };

        unsafe { spinner.stopAnimation(None) };
        window.orderOut(None);
    }
}
