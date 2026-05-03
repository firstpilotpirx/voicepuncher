use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication,
    NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSControlSize, NSFont,
    NSProgressIndicator, NSProgressIndicatorStyle, NSScreen, NSTextAlignment, NSTextField,
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

pub struct MacosVisualIndicator {
    state: Arc<Mutex<OverlayState>>,
    window_ptr: usize,
    label_ptr: usize,
    spinner_ptr: usize,
    pulse_generation: Arc<AtomicU64>,
}

impl MacosVisualIndicator {
    pub fn initialize(mtm: MainThreadMarker) -> Self {
        let window_width: f64 = 112.0;
        let window_height: f64 = 44.0;
        let menu_bar_height: f64 = 24.0;

        let screen_frame = NSScreen::mainScreen(mtm)
            .map(|s| s.frame())
            .unwrap_or_else(|| objc2_foundation::NSRect {
                origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
                size: objc2_foundation::NSSize {
                    width: 1440.0,
                    height: 900.0,
                },
            });

        let x = (screen_frame.size.width - window_width) / 2.0;
        let y = screen_frame.size.height - menu_bar_height - window_height - 4.0;

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

        let content_view = window.contentView().unwrap();
        content_view.setWantsLayer(true);

        if let Some(layer) = content_view.layer() {
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.08, 0.08, 0.08, 0.88);
            layer.setBackgroundColor(Some(&bg.CGColor()));
            layer.setCornerRadius(22.0);
            layer.setMasksToBounds(true);
        }

        let label_height: f64 = 34.0;
        let label_frame = objc2_foundation::NSRect {
            origin: objc2_foundation::NSPoint {
                x: 8.0,
                y: (window_height - label_height) / 2.0,
            },
            size: objc2_foundation::NSSize {
                width: window_width - 16.0,
                height: label_height,
            },
        };

        let label = NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), label_frame);
        label.setBezeled(false);
        label.setEditable(false);
        label.setDrawsBackground(false);
        label.setAlignment(NSTextAlignment::Center);
        let font = NSFont::systemFontOfSize(28.0);
        label.setFont(Some(&font));
        label.setTextColor(Some(&NSColor::whiteColor()));
        let empty = NSString::from_str("");
        label.setStringValue(&empty);

        content_view.addSubview(&label);

        let spinner_size: f64 = 26.0;
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
        if let Some(appearance) = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua })
        {
            spinner.setAppearance(Some(&appearance));
        }
        spinner.setHidden(true);

        content_view.addSubview(&spinner);

        let window_ptr = Retained::as_ptr(&window) as usize;
        let label_ptr = Retained::as_ptr(&label) as usize;
        let spinner_ptr = Retained::as_ptr(&spinner) as usize;

        let _ = Retained::into_raw(window);
        let _ = Retained::into_raw(label);
        let _ = Retained::into_raw(spinner);

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        Self {
            state: Arc::new(Mutex::new(OverlayState::Hidden)),
            window_ptr,
            label_ptr,
            spinner_ptr,
            pulse_generation: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        let generation = self.pulse_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pulse_gen = Arc::clone(&self.pulse_generation);
        let window_ptr = self.window_ptr;
        let label_ptr = self.label_ptr;
        let spinner_ptr = self.spinner_ptr;

        *self.state.lock().unwrap() = OverlayState::Recording;

        dispatch_to_main(move || {
            apply_recording_state(window_ptr, label_ptr, spinner_ptr, generation, pulse_gen);
        });
    }

    fn show_processing(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let window_ptr = self.window_ptr;
        let label_ptr = self.label_ptr;
        let spinner_ptr = self.spinner_ptr;

        *self.state.lock().unwrap() = OverlayState::Processing;

        dispatch_to_main(move || {
            apply_processing_state(window_ptr, label_ptr, spinner_ptr);
        });
    }

    fn hide(&self) {
        self.pulse_generation.fetch_add(1, Ordering::SeqCst);
        let window_ptr = self.window_ptr;
        let spinner_ptr = self.spinner_ptr;

        *self.state.lock().unwrap() = OverlayState::Hidden;

        dispatch_to_main(move || {
            apply_hidden_state(window_ptr, spinner_ptr);
        });
    }
}

unsafe impl Send for MacosVisualIndicator {}

fn apply_recording_state(
    window_ptr: usize,
    label_ptr: usize,
    spinner_ptr: usize,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
) {
    let window = unsafe { (window_ptr as *const NSWindow).as_ref().unwrap() };
    let label = unsafe { (label_ptr as *const NSTextField).as_ref().unwrap() };
    let spinner = unsafe {
        (spinner_ptr as *const NSProgressIndicator)
            .as_ref()
            .unwrap()
    };

    let text = NSString::from_str("◉ REC");
    label.setStringValue(&text);
    label.setTextColor(Some(&NSColor::redColor()));
    label.setHidden(false);
    spinner.setHidden(true);
    unsafe { spinner.stopAnimation(None) };
    window.orderFrontRegardless();

    schedule_pulse(window_ptr, label_ptr, generation, pulse_gen, true);
}

fn schedule_pulse(
    window_ptr: usize,
    label_ptr: usize,
    generation: u64,
    pulse_gen: Arc<AtomicU64>,
    bright: bool,
) {
    let pulse_ms: i64 = 600;
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, pulse_ms * NSEC_PER_SEC / 1000) };
    let data = Box::new((window_ptr, label_ptr, generation, pulse_gen, bright));
    let ctx = Box::into_raw(data) as *mut std::ffi::c_void;
    unsafe {
        dispatch_after_f(when, main_queue(), ctx, pulse_trampoline);
    }
}

unsafe extern "C" fn pulse_trampoline(ptr: *mut std::ffi::c_void) {
    let data: Box<(usize, usize, u64, Arc<AtomicU64>, bool)> =
        Box::from_raw(ptr as *mut (usize, usize, u64, Arc<AtomicU64>, bool));
    let (window_ptr, label_ptr, generation, pulse_gen, bright) = *data;

    if pulse_gen.load(Ordering::SeqCst) != generation {
        return;
    }

    let label = (label_ptr as *const NSTextField).as_ref().unwrap();
    let alpha: f64 = if bright { 1.0 } else { 0.35 };
    label.setAlphaValue(alpha);

    schedule_pulse(window_ptr, label_ptr, generation, pulse_gen, !bright);
}

fn apply_processing_state(window_ptr: usize, label_ptr: usize, spinner_ptr: usize) {
    let window = unsafe { (window_ptr as *const NSWindow).as_ref().unwrap() };
    let label = unsafe { (label_ptr as *const NSTextField).as_ref().unwrap() };
    let spinner = unsafe {
        (spinner_ptr as *const NSProgressIndicator)
            .as_ref()
            .unwrap()
    };

    label.setHidden(true);
    spinner.setHidden(false);
    unsafe { spinner.startAnimation(None) };
    window.orderFrontRegardless();
}

fn apply_hidden_state(window_ptr: usize, spinner_ptr: usize) {
    let window = unsafe { (window_ptr as *const NSWindow).as_ref().unwrap() };
    let spinner = unsafe {
        (spinner_ptr as *const NSProgressIndicator)
            .as_ref()
            .unwrap()
    };

    unsafe { spinner.stopAnimation(None) };
    window.orderOut(None);
}
