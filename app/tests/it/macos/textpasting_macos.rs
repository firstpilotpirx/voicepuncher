//! The macOS paster on a pasteboard of each test's own, made with a unique name: whatever was copied is
//! back after the paste, and the user's general pasteboard is never touched, so the tests run beside each
//! other and beside the user. The paste keystroke is replaced by one that sends nothing, so no
//! Accessibility permission is needed.

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString};
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::application::TextPaster;
use voicepuncher::core::domain::CleanedText;

const RESTORE_DELAY_MS: u64 = 50;
const RESTORE_DEADLINE: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PASTED_TEXT: &str = "voicepuncher-pasted-text";
const PLAIN_TEXT_TYPE: &str = "public.utf8-plain-text";
const RICH_TEXT_TYPE: &str = "public.rtf";
const PNG_TYPE: &str = "public.png";
const SCREENSHOT_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nvoicepuncher-screenshot";
const RICH_TEXT_BYTES: &[u8] = br"{\rtf1\ansi copied {\b text}}";

use voicepuncher::adapters::secondary::textpasting::macos::pasteboard::{
    build_item, read_item, ItemData as PasteboardItemTypes,
};

/// One pasteboard item holding `text` as plain text.
fn plain_text(text: &[u8]) -> Vec<PasteboardItemTypes> {
    vec![vec![(PLAIN_TEXT_TYPE.to_owned(), text.to_vec())]]
}

/// A pasteboard of the test's own, emptied when the test ends.
struct OwnPasteboard(Retained<NSPasteboard>);

impl OwnPasteboard {
    fn new() -> Self {
        Self(NSPasteboard::pasteboardWithUniqueName())
    }

    fn name(&self) -> String {
        self.0.name().to_string()
    }
}

impl Drop for OwnPasteboard {
    fn drop(&mut self) {
        self.0.clearContents();
    }
}

fn read_items(pasteboard: &OwnPasteboard) -> Vec<PasteboardItemTypes> {
    let Some(items) = pasteboard.0.pasteboardItems() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item: Retained<NSPasteboardItem>| read_item(&item))
        .collect()
}

fn write_items(pasteboard: &OwnPasteboard, items: &[PasteboardItemTypes]) {
    let pasteboard = &pasteboard.0;
    pasteboard.clearContents();
    if items.is_empty() {
        return;
    }
    let written: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
        .iter()
        .map(|types: &PasteboardItemTypes| ProtocolObject::from_retained(build_item(types)))
        .collect();
    assert!(pasteboard.writeObjects(&NSArray::from_retained_slice(&written)));
}

fn copied_text(pasteboard: &OwnPasteboard) -> Option<String> {
    let text: Retained<NSString> = pasteboard
        .0
        .stringForType(&NSString::from_str(PLAIN_TEXT_TYPE))?;
    Some(text.to_string())
}

/// Pastes and returns the paster, whose restore is pending until the delay ran out or it drops.
fn paste_without_keystroke(pasteboard: &OwnPasteboard, restore_delay_ms: u64) -> MacOsTextPaster {
    let mut paster = MacOsTextPaster::with_paste_keystroke(restore_delay_ms, || Ok(()))
        .with_pasteboard(&pasteboard.name());
    paster
        .paste(&CleanedText::new(PASTED_TEXT).unwrap())
        .unwrap();
    assert_eq!(
        copied_text(pasteboard).as_deref(),
        Some(PASTED_TEXT),
        "the paste must put its text on the pasteboard"
    );
    paster
}

fn wait_until_pasted_text_is_gone(pasteboard: &OwnPasteboard) {
    let deadline = Instant::now() + RESTORE_DEADLINE;
    while copied_text(pasteboard).as_deref() == Some(PASTED_TEXT) {
        assert!(
            Instant::now() < deadline,
            "the pasteboard was not restored within {RESTORE_DEADLINE:?}"
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Waits until every copied item is back: the restore clears the pasteboard before it writes the items, so
/// the pasted text being gone alone may still show the empty pasteboard between the two.
fn wait_until_restored(pasteboard: &OwnPasteboard, copied: &[PasteboardItemTypes]) {
    let deadline = Instant::now() + RESTORE_DEADLINE;
    while !every_type_is_back(copied, &read_items(pasteboard)) && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn should_bring_back_a_copied_screenshot_after_the_paste() {
    // Given
    let pasteboard = OwnPasteboard::new();
    let screenshot: Vec<PasteboardItemTypes> =
        vec![vec![(PNG_TYPE.to_owned(), SCREENSHOT_BYTES.to_vec())]];
    write_items(&pasteboard, &screenshot);
    // When
    let _paster = paste_without_keystroke(&pasteboard, RESTORE_DELAY_MS);
    wait_until_restored(&pasteboard, &screenshot);
    // Then
    assert_every_type_is_back(&pasteboard, &screenshot);
}

#[test]
fn should_bring_back_copied_text_with_every_type_after_the_paste() {
    // Given
    let pasteboard = OwnPasteboard::new();
    let rich_text: Vec<PasteboardItemTypes> = vec![vec![
        (PLAIN_TEXT_TYPE.to_owned(), b"copied text".to_vec()),
        (RICH_TEXT_TYPE.to_owned(), RICH_TEXT_BYTES.to_vec()),
    ]];
    write_items(&pasteboard, &rich_text);
    // When
    let _paster = paste_without_keystroke(&pasteboard, RESTORE_DELAY_MS);
    wait_until_restored(&pasteboard, &rich_text);
    // Then
    assert_every_type_is_back(&pasteboard, &rich_text);
}

/// Each copied item is back with each of its types' data; macOS may add types it converts the text to.
fn assert_every_type_is_back(pasteboard: &OwnPasteboard, copied: &[PasteboardItemTypes]) {
    let restored: Vec<PasteboardItemTypes> = read_items(pasteboard);
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
    let pasteboard = OwnPasteboard::new();
    write_items(&pasteboard, &[]);
    // When
    let _paster = paste_without_keystroke(&pasteboard, RESTORE_DELAY_MS);
    wait_until_pasted_text_is_gone(&pasteboard);
    // Then
    assert_eq!(read_items(&pasteboard), Vec::<PasteboardItemTypes>::new());
}

#[test]
fn should_not_overwrite_a_copy_made_after_the_paste() {
    // Given
    const SLOW_RESTORE_DELAY_MS: u64 = 300;
    const AFTER_RESTORE_WINDOW: Duration = Duration::from_millis(3 * SLOW_RESTORE_DELAY_MS);
    let pasteboard = OwnPasteboard::new();
    write_items(&pasteboard, &plain_text(b"copied before the paste"));
    let _paster = paste_without_keystroke(&pasteboard, SLOW_RESTORE_DELAY_MS);
    // When
    let copied_after = plain_text(b"copied after the paste");
    write_items(&pasteboard, &copied_after);
    std::thread::sleep(AFTER_RESTORE_WINDOW);
    // Then
    assert_eq!(read_items(&pasteboard), copied_after);
}

#[test]
fn should_restore_the_clipboard_at_once_when_the_app_quits_before_the_delay() {
    // Given — a paste whose restore waits far longer than the test
    let pasteboard = OwnPasteboard::new();
    let copied = plain_text(b"copied before quit");
    write_items(&pasteboard, &copied);
    let paster = paste_without_keystroke(&pasteboard, 60_000);
    // When — the stopping runtime drops its pipeline's paster
    drop(paster);
    // Then
    assert_every_type_is_back(&pasteboard, &copied);
}

#[test]
fn should_restore_the_users_clipboard_when_a_second_paste_comes_within_the_delay() {
    // Given — a paste whose restore is still pending
    let pasteboard = OwnPasteboard::new();
    let copied = plain_text(b"copied before both pastes");
    write_items(&pasteboard, &copied);
    let mut paster = paste_without_keystroke(&pasteboard, 60_000);
    // When — the next dictation pastes, then the app quits
    paster
        .paste(&CleanedText::new(PASTED_TEXT).unwrap())
        .unwrap();
    drop(paster);
    // Then — the user's copy is back, not the first dictation
    assert_every_type_is_back(&pasteboard, &copied);
}

#[test]
fn should_leave_the_general_pasteboard_alone_when_pasting_through_another() {
    // Given
    let pasteboard = OwnPasteboard::new();
    let general_before = NSPasteboard::generalPasteboard().changeCount();
    // When
    let paster = paste_without_keystroke(&pasteboard, 60_000);
    drop(paster);
    // Then
    assert_eq!(
        NSPasteboard::generalPasteboard().changeCount(),
        general_before
    );
}
