//! The `AppKit` building blocks the permissions and download windows share.

use std::cell::RefCell;
use std::thread::LocalKey;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSFont, NSTextAlignment, NSTextField, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString};

/// What Quit of a window does.
pub type QuitAction = Box<dyn Fn()>;

/// A window an adapter keeps on the main thread, with what its Quit button does.
pub trait KeptWindow {
    /// The window itself.
    fn window(&self) -> &NSWindow;
    /// What Quit does, when it was set.
    fn quit_action(&mut self) -> &mut Option<QuitAction>;
}

/// Declares `WINDOW`, the main thread's slot of the kept window `$shown`, whose fields
/// `window` and `quit` hold the window and what its Quit does.
macro_rules! kept_window_slot {
    ($shown:ident) => {
        thread_local! {
            /// The window, reached by the work dispatched to the main queue.
            static WINDOW: std::cell::RefCell<Option<$shown>> =
                const { std::cell::RefCell::new(None) };
        }

        impl $crate::adapters::appkit::KeptWindow for $shown {
            fn window(&self) -> &objc2_app_kit::NSWindow {
                &self.window
            }

            fn quit_action(&mut self) -> &mut Option<$crate::adapters::appkit::QuitAction> {
                &mut self.quit
            }
        }
    };
}
pub(crate) use kept_window_slot;

/// The main thread's slot of a kept window.
pub type WindowSlot<W> = LocalKey<RefCell<Option<W>>>;

/// Sets what Quit of the window in `slot` does.
pub fn set_quit<W: KeptWindow>(slot: &'static WindowSlot<W>, quit: QuitAction) {
    slot.with_borrow_mut(|state| {
        if let Some(state) = state {
            *state.quit_action() = Some(quit);
        }
    });
}

/// Runs what Quit of the window in `slot` does; exits the process when nothing was set.
pub fn quit<W: KeptWindow>(slot: &'static WindowSlot<W>) {
    let quit = slot.with_borrow_mut(|state| state.as_mut().and_then(|s| s.quit_action().take()));
    match quit {
        Some(quit) => quit(),
        None => std::process::exit(0),
    }
}

/// Hides the window in `slot`.
pub fn close<W: KeptWindow>(slot: &'static WindowSlot<W>) {
    slot.with_borrow(|state| {
        if let Some(state) = state {
            state.window().orderOut(None);
        }
    });
}

/// A new object of `T`, a class defined on `NSObject` without instance variables.
pub fn new_plain_object<T>(mtm: MainThreadMarker) -> Retained<T>
where
    T: DefinedClass<Ivars = ()> + ClassType<Super = NSObject> + MainThreadOnly,
{
    let this = T::alloc(mtm).set_ivars(());
    // SAFETY: `NSObject`'s `init` on a freshly allocated object.
    unsafe { msg_send![super(this), init] }
}

/// An empty content view `width` by `height`.
pub fn content_view(mtm: MainThreadMarker, width: f64, height: f64) -> Retained<NSView> {
    NSView::initWithFrame(
        mtm.alloc::<NSView>(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height)),
    )
}

/// A titled window with a close button, `width` by `height`, kept when closed.
pub fn titled_window(
    mtm: MainThreadMarker,
    title: &str,
    width: f64,
    height: f64,
) -> Retained<NSWindow> {
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    // SAFETY: a new titled window; it is not released on close, the state keeps it.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            frame,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: the window is kept by its owner, so it must not release itself on close.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str(title));
    window
}

/// Frame `height` high, `y` points below the top edge of content `content_height` high.
pub fn below_top(content_height: f64, x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, content_height - y - height),
        NSSize::new(width, height),
    )
}

/// A one-line label, left aligned.
pub fn label(mtm: MainThreadMarker, text: &str, font: &NSFont) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    field.setFont(Some(font));
    field.setAlignment(NSTextAlignment::Left);
    field
}

/// A label that wraps onto several lines.
pub fn wrapping(mtm: MainThreadMarker, text: &str, font: &NSFont) -> Retained<NSTextField> {
    let field = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
    field.setFont(Some(font));
    field
}

/// A push button that calls `action` of `target`.
pub fn push_button(
    mtm: MainThreadMarker,
    target: &AnyObject,
    title: &str,
    action: Sel,
) -> Retained<NSButton> {
    // SAFETY: `action` is a method of `target` taking the sender, as a button calls it.
    unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    }
}
