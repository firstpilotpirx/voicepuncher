//! The `AppKit` building blocks the permissions and download windows share.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSFont, NSTextAlignment, NSTextField, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

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
