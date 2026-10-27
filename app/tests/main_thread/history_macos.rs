//! Integration tests for the history window against `AppKit`: the pasteboard it copies to,
//! the double-click on a row, one window however often it opens, and the check that lists
//! a new dictation while the window is open.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSPasteboard, NSPasteboardTypeString, NSScrollView,
    NSTableView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSIndexSet, NSPoint, NSRect, NSSize, NSString};
use voicepuncher::adapters::primary::history::macos::{
    bring_to_front, pasteboard_copier, HistoryWindow, WINDOW_TITLE,
};
use voicepuncher::adapters::primary::history::HistoryParts;
use voicepuncher::core::application::{DictationHistory, HistoryList, HistoryRow};

use voicepuncher_test_steps::required_env;

use crate::{main_thread, Test};

pub const TESTS: &[Test] = &[
    (
        "should_copy_a_text_to_a_named_pasteboard",
        should_copy_a_text_to_a_named_pasteboard,
    ),
    (
        "needs_desktop::should_copy_the_double_clicked_row",
        should_copy_the_double_clicked_row,
    ),
    (
        "needs_desktop::should_keep_one_window_when_opened_twice",
        should_keep_one_window_when_opened_twice,
    ),
    (
        "needs_desktop::should_list_a_new_dictation_while_open",
        should_list_a_new_dictation_while_open,
    ),
];

/// How long the main queue gets to show what was dispatched to it.
const DRAW: f64 = 0.3;
/// Longer than the window's check interval.
const PAST_A_CHECK: f64 = 1.5;

/// A history whose rows the test sets, newest first.
#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<HistoryRow>>>);

impl Rows {
    fn add(&self, second: u64, text: &str) {
        let started_at = UNIX_EPOCH + Duration::from_secs(1_792_406_520 + second);
        let row = HistoryRow { started_at, text: text.to_owned() };
        self.0.lock().unwrap().insert(0, row);
    }
}

impl DictationHistory for Rows {
    fn latest(&self, limit: usize) -> Result<HistoryList> {
        Ok(HistoryList::Rows(
            self.0.lock().unwrap().iter().take(limit).cloned().collect(),
        ))
    }

    fn newest_entry(&self) -> Option<String> {
        let rows = self.0.lock().unwrap();
        rows.first().map(|row| format!("{:?}", row.started_at))
    }
}

/// Installs the window over `rows`; what it copies lands in the returned list.
fn install(rows: &Rows) -> Arc<Mutex<Vec<String>>> {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let (rows, sink) = (rows.clone(), Arc::clone(&copied));
    HistoryWindow::install(
        main_thread(),
        Box::new(move || {
            let (rows, sink) = (rows.clone(), Arc::clone(&sink));
            HistoryParts {
                source: Box::new(move || Box::new(rows.clone())),
                copier: Box::new(move |text| {
                    sink.lock().unwrap().push(text.to_owned());
                    Ok(())
                }),
                utc_offset: Box::new(|_: SystemTime| 0),
            }
        }),
    );
    copied
}

fn pump_run_loop(secs: f64) {
    use objc2_foundation::{NSDate, NSRunLoop};
    let deadline = NSDate::dateWithTimeIntervalSinceNow(secs);
    NSRunLoop::currentRunLoop().runUntilDate(&deadline);
}

fn history_windows() -> Vec<Retained<NSWindow>> {
    let windows = NSApplication::sharedApplication(main_thread()).windows();
    windows
        .iter()
        .filter(|w| w.title().to_string() == WINDOW_TITLE)
        .collect()
}

/// A plain titled window that is not the history window.
fn other_window() -> Retained<NSWindow> {
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 100.0));
    // SAFETY: a new titled window; the test keeps it and closes it at its end.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            main_thread().alloc::<NSWindow>(),
            frame,
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: the test owns the window and closes it; AppKit must not release it on close too.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str("Another window"));
    window
}

/// The title of the frontmost window of the app.
fn front_title() -> Option<String> {
    let windows = NSApplication::sharedApplication(main_thread()).orderedWindows();
    windows
        .iter()
        .find(|w| w.isVisible())
        .map(|w| w.title().to_string())
}

fn table_of(window: &NSWindow) -> Retained<NSTableView> {
    let content = window.contentView().expect("the window has content");
    let scroll = content
        .subviews()
        .firstObject()
        .expect("the window shows a table");
    let scroll = scroll
        .downcast::<NSScrollView>()
        .expect("the table scrolls");
    let table = scroll
        .documentView()
        .expect("the scroll view holds the table");
    table.downcast::<NSTableView>().expect("a table view")
}

fn should_copy_a_text_to_a_named_pasteboard() {
    // Given
    let _mtm = main_thread();
    let name = format!("voicepuncher-history-test-{}", std::process::id());
    let pasteboard = NSPasteboard::pasteboardWithName(&NSString::from_str(&name));
    // SAFETY: `NSPasteboardTypeString` is a constant AppKit defines and never changes.
    let plain_text = unsafe { NSPasteboardTypeString };
    pasteboard.clearContents();
    pasteboard.setString_forType(&NSString::from_str("what was there before"), plain_text);
    let mut copy = pasteboard_copier(Some(name));
    // When
    copy("Hello world.").unwrap();
    // Then
    let text = pasteboard.stringForType(plain_text).map(|s| s.to_string());
    assert_eq!(text.as_deref(), Some("Hello world."));
}

fn should_copy_the_double_clicked_row() {
    // Given
    required_env("VOICEPUNCHER_DESKTOP_IT");
    let rows = Rows::default();
    rows.add(1, "first dictation");
    rows.add(2, "second dictation");
    let copied = install(&rows);
    bring_to_front();
    pump_run_loop(DRAW);
    let window = history_windows().pop().expect("the history window opened");
    let table = table_of(&window);
    // When
    table.selectRowIndexes_byExtendingSelection(&NSIndexSet::indexSetWithIndex(1), false);
    let app = NSApplication::sharedApplication(main_thread());
    // SAFETY: the table's double action and target were set by the window, which is alive.
    let sent = unsafe {
        app.sendAction_to_from(
            table.doubleAction().unwrap(),
            table.target().as_deref(),
            Some(&table),
        )
    };
    // Then
    window.close();
    assert!(sent, "the double action reached no target");
    assert_eq!(*copied.lock().unwrap(), ["first dictation"]);
}

fn should_keep_one_window_when_opened_twice() {
    // Given
    required_env("VOICEPUNCHER_DESKTOP_IT");
    let rows = Rows::default();
    rows.add(1, "a dictation");
    install(&rows);
    bring_to_front();
    pump_run_loop(DRAW);
    let other = other_window();
    other.makeKeyAndOrderFront(None);
    pump_run_loop(DRAW);
    // When
    bring_to_front();
    pump_run_loop(DRAW);
    // Then
    let windows = history_windows();
    let in_front = front_title();
    windows.iter().for_each(|w| w.close());
    other.close();
    assert_eq!(windows.len(), 1);
    assert_eq!(in_front.as_deref(), Some(WINDOW_TITLE));
}

fn should_list_a_new_dictation_while_open() {
    // Given
    required_env("VOICEPUNCHER_DESKTOP_IT");
    let rows = Rows::default();
    rows.add(1, "a dictation");
    install(&rows);
    bring_to_front();
    pump_run_loop(DRAW);
    let window = history_windows().pop().expect("the history window opened");
    // When
    rows.add(2, "a newer dictation");
    pump_run_loop(PAST_A_CHECK);
    // Then
    let shown = table_of(&window).numberOfRows();
    window.close();
    assert_eq!(shown, 2);
}
