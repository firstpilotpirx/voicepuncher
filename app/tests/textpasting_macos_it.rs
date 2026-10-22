//! The macOS paster on the real general pasteboard: whatever the user had copied is back after the paste.
//! The paste keystroke is replaced by one that sends nothing, so no Accessibility permission is needed.
//! Each test takes the machine-wide pasteboard lock first, so runs from parallel checkouts take turns, then
//! saves the user's pasteboard and puts it back when it ends, before the lock is released; nextest runs this
//! binary's tests one at a time (`.config/nextest.toml`, test group `pasteboard`).

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::application::TextPaster;
use voicepuncher::core::domain::CleanedText;

mod support;
use support::pasteboard_lock::PasteboardLock;

const RESTORE_DELAY_MS: u64 = 50;
const RESTORE_DEADLINE: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PASTED_TEXT: &str = "voicepuncher-pasted-text";
const PLAIN_TEXT_TYPE: &str = "public.utf8-plain-text";
const RICH_TEXT_TYPE: &str = "public.rtf";
const PNG_TYPE: &str = "public.png";
const SCREENSHOT_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nvoicepuncher-screenshot";
const RICH_TEXT_BYTES: &[u8] = br"{\rtf1\ansi copied {\b text}}";

type PasteboardItemTypes = Vec<(String, Vec<u8>)>;

/// One pasteboard item holding `text` as plain text.
fn plain_text(text: &[u8]) -> Vec<PasteboardItemTypes> {
    vec![vec![(PLAIN_TEXT_TYPE.to_owned(), text.to_vec())]]
}

fn send_no_keystroke() -> anyhow::Result<()> {
    Ok(())
}

fn general_pasteboard() -> Retained<NSPasteboard> {
    NSPasteboard::generalPasteboard()
}

fn read_items() -> Vec<PasteboardItemTypes> {
    let Some(items) = general_pasteboard().pasteboardItems() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item: Retained<NSPasteboardItem>| {
            item.types()
                .iter()
                .filter_map(|data_type: Retained<NSString>| {
                    let data: Retained<NSData> = item.dataForType(&data_type)?;
                    Some((data_type.to_string(), data.to_vec()))
                })
                .collect()
        })
        .collect()
}

fn write_items(items: &[PasteboardItemTypes]) {
    let pasteboard = general_pasteboard();
    pasteboard.clearContents();
    if items.is_empty() {
        return;
    }
    let written: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
        .iter()
        .map(|types: &PasteboardItemTypes| {
            let item: Retained<NSPasteboardItem> = NSPasteboardItem::new();
            for (data_type, bytes) in types {
                item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(data_type));
            }
            ProtocolObject::from_retained(item)
        })
        .collect();
    assert!(pasteboard.writeObjects(&NSArray::from_retained_slice(&written)));
}

fn copied_text() -> Option<String> {
    let pasteboard = general_pasteboard();
    let text: Retained<NSString> =
        pasteboard.stringForType(&NSString::from_str(PLAIN_TEXT_TYPE))?;
    Some(text.to_string())
}

/// The user's pasteboard, put back when the test ends, pass or fail.
struct SavedUserPasteboard(Vec<PasteboardItemTypes>);

impl SavedUserPasteboard {
    fn save() -> Self {
        Self(read_items())
    }
}

impl Drop for SavedUserPasteboard {
    fn drop(&mut self) {
        write_items(&self.0);
    }
}

/// Pastes and returns the paster, whose restore is pending until the delay ran out or it drops.
fn paste_without_keystroke(restore_delay_ms: u64) -> MacOsTextPaster {
    let mut paster = MacOsTextPaster::with_paste_keystroke(restore_delay_ms, send_no_keystroke);
    paster
        .paste(&CleanedText::new(PASTED_TEXT.to_owned()).unwrap())
        .unwrap();
    assert_eq!(
        copied_text().as_deref(),
        Some(PASTED_TEXT),
        "the paste must put its text on the pasteboard"
    );
    paster
}

fn wait_until_pasted_text_is_gone() {
    let deadline = Instant::now() + RESTORE_DEADLINE;
    while copied_text().as_deref() == Some(PASTED_TEXT) {
        assert!(
            Instant::now() < deadline,
            "the pasteboard was not restored within {RESTORE_DEADLINE:?}"
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Waits until every copied item is back: the restore clears the pasteboard before it writes the items, so
/// the pasted text being gone alone may still show the empty pasteboard between the two.
fn wait_until_restored(copied: &[PasteboardItemTypes]) {
    let deadline = Instant::now() + RESTORE_DEADLINE;
    while !every_type_is_back(copied, &read_items()) && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn should_bring_back_a_copied_screenshot_after_the_paste() {
    // Given
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    let screenshot: Vec<PasteboardItemTypes> =
        vec![vec![(PNG_TYPE.to_owned(), SCREENSHOT_BYTES.to_vec())]];
    write_items(&screenshot);
    // When
    let _paster = paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_restored(&screenshot);
    // Then
    assert_every_type_is_back(&screenshot);
}

#[test]
fn should_bring_back_copied_text_with_every_type_after_the_paste() {
    // Given
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    let rich_text: Vec<PasteboardItemTypes> = vec![vec![
        (PLAIN_TEXT_TYPE.to_owned(), b"copied text".to_vec()),
        (RICH_TEXT_TYPE.to_owned(), RICH_TEXT_BYTES.to_vec()),
    ]];
    write_items(&rich_text);
    // When
    let _paster = paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_restored(&rich_text);
    // Then
    assert_every_type_is_back(&rich_text);
}

/// Each copied item is back with each of its types' data; macOS may add types it converts the text to.
fn assert_every_type_is_back(copied: &[PasteboardItemTypes]) {
    let restored: Vec<PasteboardItemTypes> = read_items();
    assert!(
        every_type_is_back(copied, &restored),
        "{copied:?} not all in {restored:?}"
    );
}

fn every_type_is_back(copied: &[PasteboardItemTypes], restored: &[PasteboardItemTypes]) -> bool {
    restored.len() == copied.len()
        && copied
            .iter()
            .zip(restored)
            .all(|(copied_item, restored_item)| {
                copied_item
                    .iter()
                    .all(|copied_type| restored_item.contains(copied_type))
            })
}

#[test]
fn should_leave_an_empty_clipboard_empty_after_the_paste() {
    // Given
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    write_items(&[]);
    // When
    let _paster = paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_pasted_text_is_gone();
    // Then
    assert_eq!(read_items(), Vec::<PasteboardItemTypes>::new());
}

#[test]
fn should_not_overwrite_a_copy_made_after_the_paste() {
    // Given
    const SLOW_RESTORE_DELAY_MS: u64 = 300;
    const AFTER_RESTORE_WINDOW: Duration = Duration::from_millis(3 * SLOW_RESTORE_DELAY_MS);
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    write_items(&plain_text(b"copied before the paste"));
    let _paster = paste_without_keystroke(SLOW_RESTORE_DELAY_MS);
    // When
    let copied_after = plain_text(b"copied after the paste");
    write_items(&copied_after);
    std::thread::sleep(AFTER_RESTORE_WINDOW);
    // Then
    assert_eq!(read_items(), copied_after);
}

#[test]
fn should_make_a_second_run_wait_while_another_holds_the_pasteboard() {
    // Given
    const NOT_TAKEN_FOR: Duration = Duration::from_millis(200);
    const TAKEN_WITHIN: Duration = Duration::from_secs(2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pasteboard.lock");
    let first_run = PasteboardLock::take_at(&path);
    let (taken_tx, taken_rx) = std::sync::mpsc::channel::<()>();
    let second_path = path;
    let second_run = std::thread::spawn(move || {
        let _lock = PasteboardLock::take_at(&second_path);
        taken_tx.send(()).unwrap();
    });
    // When
    let taken_while_held = taken_rx.recv_timeout(NOT_TAKEN_FOR).is_ok();
    drop(first_run);
    // Then
    assert!(
        !taken_while_held,
        "the second run took the pasteboard lock while the first held it"
    );
    taken_rx
        .recv_timeout(TAKEN_WITHIN)
        .expect("the second run must take the lock once the first drops it");
    second_run.join().unwrap();
}

#[test]
fn should_restore_the_clipboard_at_once_when_the_app_quits_before_the_delay() {
    // Given — a paste whose restore waits far longer than the test
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    let copied = plain_text(b"copied before quit");
    write_items(&copied);
    let paster = paste_without_keystroke(60_000);
    // When — the stopping runtime drops its pipeline's paster
    drop(paster);
    // Then
    assert_every_type_is_back(&copied);
}

#[test]
fn should_restore_the_users_clipboard_when_a_second_paste_comes_within_the_delay() {
    // Given — a paste whose restore is still pending
    let _pasteboard_lock = PasteboardLock::take();
    let _user_pasteboard = SavedUserPasteboard::save();
    let copied = plain_text(b"copied before both pastes");
    write_items(&copied);
    let mut paster = paste_without_keystroke(60_000);
    // When — the next dictation pastes, then the app quits
    paster
        .paste(&CleanedText::new(PASTED_TEXT.to_owned()).unwrap())
        .unwrap();
    drop(paster);
    // Then — the user's copy is back, not the first dictation
    assert_every_type_is_back(&copied);
}
