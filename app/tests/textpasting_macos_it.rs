//! The macOS paster on the real general pasteboard: whatever the user had copied is back after the paste.
//! The paste keystroke is replaced by one that sends nothing, so no Accessibility permission is needed.
//! Each test saves the user's pasteboard first and puts it back when it ends; nextest runs this binary's
//! tests one at a time (`.config/nextest.toml`, test group `pasteboard`).

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
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

type PasteboardItemTypes = Vec<(String, Vec<u8>)>;

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

fn paste_without_keystroke(restore_delay_ms: u64) {
    let mut paster = MacOsTextPaster::with_paste_keystroke(restore_delay_ms, send_no_keystroke);
    paster
        .paste(&CleanedText::new(PASTED_TEXT.to_owned()).unwrap())
        .unwrap();
    assert_eq!(
        copied_text().as_deref(),
        Some(PASTED_TEXT),
        "the paste must put its text on the pasteboard"
    );
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

#[test]
fn should_bring_back_a_copied_screenshot_after_the_paste() {
    // Given
    let _user_pasteboard = SavedUserPasteboard::save();
    let screenshot: Vec<PasteboardItemTypes> =
        vec![vec![(PNG_TYPE.to_owned(), SCREENSHOT_BYTES.to_vec())]];
    write_items(&screenshot);
    // When
    paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_pasted_text_is_gone();
    // Then
    assert_every_type_is_back(&screenshot);
}

#[test]
fn should_bring_back_copied_text_with_every_type_after_the_paste() {
    // Given
    let _user_pasteboard = SavedUserPasteboard::save();
    let rich_text: Vec<PasteboardItemTypes> = vec![vec![
        (PLAIN_TEXT_TYPE.to_owned(), b"copied text".to_vec()),
        (RICH_TEXT_TYPE.to_owned(), RICH_TEXT_BYTES.to_vec()),
    ]];
    write_items(&rich_text);
    // When
    paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_pasted_text_is_gone();
    // Then
    assert_every_type_is_back(&rich_text);
}

/// Each copied item is back with each of its types' data; macOS may add types it converts the text to.
fn assert_every_type_is_back(copied: &[PasteboardItemTypes]) {
    let restored: Vec<PasteboardItemTypes> = read_items();
    assert_eq!(restored.len(), copied.len(), "items: {restored:?}");
    for (copied_item, restored_item) in copied.iter().zip(&restored) {
        for copied_type in copied_item {
            assert!(
                restored_item.contains(copied_type),
                "{copied_type:?} missing from {restored_item:?}"
            );
        }
    }
}

#[test]
fn should_leave_an_empty_clipboard_empty_after_the_paste() {
    // Given
    let _user_pasteboard = SavedUserPasteboard::save();
    write_items(&[]);
    // When
    paste_without_keystroke(RESTORE_DELAY_MS);
    wait_until_pasted_text_is_gone();
    // Then
    assert_eq!(read_items(), Vec::<PasteboardItemTypes>::new());
}

#[test]
fn should_not_overwrite_a_copy_made_after_the_paste() {
    // Given
    const SLOW_RESTORE_DELAY_MS: u64 = 300;
    const AFTER_RESTORE_WINDOW: Duration = Duration::from_millis(3 * SLOW_RESTORE_DELAY_MS);
    let _user_pasteboard = SavedUserPasteboard::save();
    write_items(&[vec![(
        PLAIN_TEXT_TYPE.to_owned(),
        b"copied before the paste".to_vec(),
    )]]);
    paste_without_keystroke(SLOW_RESTORE_DELAY_MS);
    // When
    let copied_after: Vec<PasteboardItemTypes> = vec![vec![(
        PLAIN_TEXT_TYPE.to_owned(),
        b"copied after the paste".to_vec(),
    )]];
    write_items(&copied_after);
    std::thread::sleep(AFTER_RESTORE_WINDOW);
    // Then
    assert_eq!(read_items(), copied_after);
}
