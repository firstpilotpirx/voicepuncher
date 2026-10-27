//! The history window in `AppKit`: one window, built on first use and brought to the front
//! afterwards; while it is visible it checks the archive once a second.

mod table;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeString, NSWindow};
use objc2_foundation::{NSDate, NSString, NSTimeZone};

use super::{Copier, HistoryParts, HistoryView, UtcOffset};
use crate::adapters::appkit;
use crate::adapters::main_queue::{dispatch_to_main, on_main_after};

use table::{build_content, TableSource};

/// The window's title, also how a test finds it.
pub const WINDOW_TITLE: &str = "Dictation history";
const WIDTH: f64 = 720.0;
const HEIGHT: f64 = 480.0;
/// How often the open window checks the archive for a new dictation.
const CHECK_EVERY_MS: i64 = 1_000;

/// Builds the parts of a window opened now: the history of the saved configuration's archive.
pub type PartsSource = Box<dyn Fn() -> HistoryParts>;

appkit::kept_window_slot!(ShownWindow);

struct ShownWindow {
    window: Retained<NSWindow>,
    source: Retained<TableSource>,
    parts: PartsSource,
    view: Option<HistoryView>,
    shown_once: bool,
    checking: bool,
    quit: Option<appkit::QuitAction>,
}

/// The history window of the app.
pub struct HistoryWindow;

impl HistoryWindow {
    /// Prepares the window, hidden until the user opens it from the menu.
    pub fn install(mtm: MainThreadMarker, parts: PartsSource) -> Self {
        let window = appkit::titled_window(mtm, WINDOW_TITLE, WIDTH, HEIGHT);
        let source = TableSource::new(mtm);
        WINDOW.set(Some(ShownWindow {
            window,
            source,
            parts,
            view: None,
            shown_once: false,
            checking: false,
            quit: None,
        }));
        Self
    }
}

/// Reads the archive again and brings the window to the front; from the menu.
pub fn bring_to_front() {
    dispatch_to_main(show);
}

fn show() {
    let start_checking = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        state.view = Some(HistoryView::open((state.parts)()));
        if !state.shown_once {
            state.window.center();
        }
        state.shown_once = true;
        Some(!std::mem::replace(&mut state.checking, true))
    });
    redraw();
    WINDOW.with_borrow(|state| {
        let Some(state) = state else { return };
        let mtm = MainThreadMarker::from(&*state.source);
        NSApplication::sharedApplication(mtm).activate();
        state.window.makeKeyAndOrderFront(None);
    });
    if start_checking == Some(true) {
        on_main_after(CHECK_EVERY_MS, |_| check());
    }
}

/// Lists a dictation archived since the last read; stops once the window is closed.
fn check() {
    let changed = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        if !state.window.isVisible() {
            state.checking = false;
            return None;
        }
        Some(state.view.as_mut().is_some_and(HistoryView::check))
    });
    match changed {
        None => return,
        Some(true) => redraw(),
        Some(false) => {}
    }
    on_main_after(CHECK_EVERY_MS, |_| check());
}

fn redraw() {
    WINDOW.with_borrow(|state| {
        let Some(state) = state else { return };
        let Some(view) = &state.view else { return };
        let mtm = MainThreadMarker::from(&*state.source);
        let content = build_content(mtm, &state.source, view.shown(), WIDTH, HEIGHT);
        state.window.setContentView(Some(&content));
    });
}

/// Copies the text of row `index`, from its Copy button or a double-click.
fn copy_row(index: usize) {
    WINDOW.with_borrow_mut(|state| {
        let Some(view) = state.as_mut().and_then(|s| s.view.as_mut()) else {
            return;
        };
        if let Err(e) = view.copy(index) {
            log::warn!("history: row {index} not copied: {e:#}");
        }
    });
}

/// Puts a row's text, as plain text, on the general pasteboard or the one `name` names.
pub fn pasteboard_copier(name: Option<String>) -> Copier {
    Box::new(move |text| {
        let pasteboard = name
            .as_deref()
            .map_or_else(NSPasteboard::generalPasteboard, |name| {
                NSPasteboard::pasteboardWithName(&NSString::from_str(name))
            });
        pasteboard.clearContents();
        // SAFETY: `NSPasteboardTypeString` is a constant AppKit defines and never changes.
        let plain_text = unsafe { NSPasteboardTypeString };
        if !pasteboard.setString_forType(&NSString::from_str(text), plain_text) {
            anyhow::bail!("the pasteboard refused the text");
        }
        Ok(())
    })
}

/// The Mac's local time zone's offset from UTC at an instant.
pub fn local_utc_offset() -> UtcOffset {
    Box::new(|instant| {
        let since_epoch = instant
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let date = NSDate::dateWithTimeIntervalSince1970(since_epoch.as_secs_f64());
        i64::try_from(NSTimeZone::localTimeZone().secondsFromGMTForDate(&date)).unwrap_or(0)
    })
}
