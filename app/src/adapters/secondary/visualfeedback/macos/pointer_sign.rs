//! The dictation sign's window: a 🎙️ that follows the mouse pointer while a recording runs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSBackingStoreType, NSColor, NSFont, NSTextAlignment, NSTextField, NSWindow};
use objc2_app_kit::{NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::{dispatch_to_main, dispatch_to_main_after};
use crate::adapters::secondary::visualfeedback::pointer_sign::PointerSignView;

const SIGN: &str = "🎙️";
/// The sign's window, square.
const SIGN_SIZE: f64 = 28.0;
/// How far right of and below the pointer's hot spot the sign's top-left corner sits.
const POINTER_GAP: f64 = 16.0;
/// How often the sign looks where the pointer went: about once per frame.
const FOLLOW_MS: i64 = 16;
/// Above app windows and the overlay, below the pointer.
const SIGN_LEVEL: isize = 101;

/// The origin, in global screen coordinates (bottom-left origin), of a sign `size` points
/// square whose top-left corner sits `POINTER_GAP` right of and below `pointer`.
pub(super) fn sign_origin(pointer: NSPoint, size: f64) -> NSPoint {
    NSPoint {
        x: pointer.x + POINTER_GAP,
        y: pointer.y - POINTER_GAP - size,
    }
}

struct SignWindow(Retained<NSWindow>);

// The window is only touched on the main thread: every use goes through the main queue.
unsafe impl Send for SignWindow {}
unsafe impl Sync for SignWindow {}

/// The dictation sign next to the pointer, over every app and on every screen.
pub struct MacosPointerSign {
    window: Arc<SignWindow>,
    /// Bumped by every show and hide; a follow tick of an older one stops.
    generation: Arc<AtomicU64>,
}

impl MacosPointerSign {
    pub fn initialize(mtm: MainThreadMarker) -> Self {
        Self {
            window: Arc::new(SignWindow(build_window(mtm))),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.0.isVisible()
    }

    pub fn ignores_mouse_events(&self) -> bool {
        self.window.0.ignoresMouseEvents()
    }

    pub fn can_become_key(&self) -> bool {
        self.window.0.canBecomeKeyWindow()
    }

    /// Where the sign's top-left corner is, in global screen coordinates.
    pub fn top_left(&self) -> NSPoint {
        let frame = self.window.0.frame();
        NSPoint {
            x: frame.origin.x,
            y: frame.origin.y + frame.size.height,
        }
    }
}

impl PointerSignView for MacosPointerSign {
    fn show(&self) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let window = Arc::clone(&self.window);
        let current = Arc::clone(&self.generation);
        dispatch_to_main(move || {
            move_to_pointer(&window.0);
            window.0.orderFrontRegardless();
            follow(window, generation, current);
        });
    }

    fn hide(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let window = Arc::clone(&self.window);
        dispatch_to_main(move || window.0.orderOut(None));
    }
}

/// Moves the sign to the pointer once per frame until a later show or hide.
fn follow(window: Arc<SignWindow>, generation: u64, current: Arc<AtomicU64>) {
    dispatch_to_main_after(FOLLOW_MS, move || {
        if current.load(Ordering::SeqCst) != generation {
            return;
        }
        move_to_pointer(&window.0);
        follow(window, generation, current);
    });
}

fn move_to_pointer(window: &NSWindow) {
    let origin = sign_origin(objc2_app_kit::NSEvent::mouseLocation(), SIGN_SIZE);
    if window.frame().origin != origin {
        window.setFrameOrigin(origin);
    }
}

fn build_window(mtm: MainThreadMarker) -> Retained<NSWindow> {
    let frame = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: SIGN_SIZE,
            height: SIGN_SIZE,
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
    unsafe { window.setReleasedWhenClosed(false) };
    window.setLevel(SIGN_LEVEL);
    window.setOpaque(false);
    window.setHasShadow(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );

    let label = NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), frame);
    label.setBezeled(false);
    label.setEditable(false);
    label.setSelectable(false);
    label.setDrawsBackground(false);
    label.setAlignment(NSTextAlignment::Center);
    label.setFont(Some(&NSFont::systemFontOfSize(20.0)));
    label.setStringValue(&NSString::from_str(SIGN));
    if let Some(content) = window.contentView() {
        content.addSubview(&label);
    }
    window
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_place_the_sign_right_of_and_below_the_pointer() {
        let origin = sign_origin(NSPoint { x: 500.0, y: 300.0 }, 28.0);
        assert_eq!(origin, NSPoint { x: 516.0, y: 256.0 });
    }

    #[test]
    fn should_place_the_sign_on_a_screen_left_of_and_below_the_main_one() {
        let origin = sign_origin(
            NSPoint {
                x: -1200.0,
                y: -40.0,
            },
            28.0,
        );
        assert_eq!(
            origin,
            NSPoint {
                x: -1184.0,
                y: -84.0
            }
        );
    }
}
