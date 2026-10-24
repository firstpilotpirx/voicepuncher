//! The macOS paster on a pasteboard of the scenario's own, made with a unique name, so the
//! user's clipboard is never touched; the paste keystroke is replaced by one that sends nothing.

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::{application::TextPaster, domain::CleanedText};

use super::{Duration, Instant, DICTATED};
use crate::IntegrationWorld;

/// The restore delay of a paste the scenario lets run out.
const SHORT_DELAY_MS: u64 = 50;
/// The restore delay of a paste the scenario acts on before it runs out.
const PENDING_DELAY_MS: u64 = 300;
const RESTORE_DEADLINE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(10);
const PLAIN_TEXT: &str = "public.utf8-plain-text";
const PNG: &str = "public.png";
const SCREENSHOT: &[u8] = b"\x89PNG\r\n\x1a\nvoicepuncher-screenshot";

/// The data of each type of one pasteboard item.
type Item = Vec<(String, Vec<u8>)>;

/// A paste scenario's pasteboard, what the clipboard must hold at its end, and the paster.
pub struct ClipboardRun {
    pasteboard: Retained<NSPasteboard>,
    expected: Vec<Item>,
    paster: Option<MacOsTextPaster>,
}

impl std::fmt::Debug for ClipboardRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardRun")
            .field("expected", &self.expected)
            .finish_non_exhaustive()
    }
}

impl Drop for ClipboardRun {
    fn drop(&mut self) {
        drop(self.paster.take());
        self.pasteboard.clearContents();
    }
}

impl ClipboardRun {
    fn copy(&mut self, items: Vec<Item>) {
        write_items(&self.pasteboard, &items);
        self.expected = items;
    }

    fn paste(&mut self, restore_delay_ms: u64) -> Result<()> {
        let mut paster = MacOsTextPaster::with_paste_keystroke(restore_delay_ms, || Ok(()))
            .with_pasteboard(&self.pasteboard.name().to_string());
        paster.paste(&CleanedText::new(DICTATED.to_owned())?)?;
        self.paster = Some(paster);
        Ok(())
    }

    /// Whether every expected item is back with the data of each of its types; macOS may add
    /// types it converts text to.
    fn restored(&self) -> bool {
        let now = read_items(&self.pasteboard);
        now.len() == self.expected.len()
            && self
                .expected
                .iter()
                .zip(&now)
                .all(|(expected, item)| expected.iter().all(|kind| item.contains(kind)))
    }
}

fn plain_text(text: &str) -> Vec<Item> {
    vec![vec![(PLAIN_TEXT.to_owned(), text.as_bytes().to_vec())]]
}

fn read_items(pasteboard: &NSPasteboard) -> Vec<Item> {
    let Some(items) = pasteboard.pasteboardItems() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item: Retained<NSPasteboardItem>| {
            item.types()
                .iter()
                .filter_map(|kind: Retained<NSString>| {
                    let data: Retained<NSData> = item.dataForType(&kind)?;
                    Some((kind.to_string(), data.to_vec()))
                })
                .collect()
        })
        .collect()
}

fn write_items(pasteboard: &NSPasteboard, items: &[Item]) {
    pasteboard.clearContents();
    if items.is_empty() {
        return;
    }
    let written: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
        .iter()
        .map(|kinds| {
            let item = NSPasteboardItem::new();
            for (kind, bytes) in kinds {
                item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(kind));
            }
            ProtocolObject::from_retained(item)
        })
        .collect();
    pasteboard.writeObjects(&NSArray::from_retained_slice(&written));
}

fn clipboard(world: &mut IntegrationWorld) -> &mut ClipboardRun {
    world
        .extra
        .insertion
        .clipboard
        .get_or_insert_with(|| ClipboardRun {
            pasteboard: NSPasteboard::pasteboardWithUniqueName(),
            expected: Vec::new(),
            paster: None,
        })
}

#[given("the user copies a screenshot to the clipboard while a recording runs")]
fn screenshot_copied(world: &mut IntegrationWorld) {
    clipboard(world).copy(vec![vec![(PNG.to_owned(), SCREENSHOT.to_vec())]]);
}

#[given("the clipboard holds text the user copied")]
fn text_copied(world: &mut IntegrationWorld) {
    clipboard(world).copy(plain_text("copied by the user"));
}

#[given("the clipboard is empty")]
fn clipboard_empty(world: &mut IntegrationWorld) {
    clipboard(world).copy(Vec::new());
}

#[given("a dictation was just pasted and its restore delay has not run out")]
fn just_pasted(world: &mut IntegrationWorld) -> Result<()> {
    clipboard(world).paste(PENDING_DELAY_MS)
}

#[when("a dictation is pasted")]
fn dictation_pasted(world: &mut IntegrationWorld) -> Result<()> {
    clipboard(world).paste(SHORT_DELAY_MS)
}

#[when("a second dictation is pasted")]
fn second_dictation_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = clipboard(world);
    let paster = run.paster.as_mut().context("no dictation was pasted")?;
    paster.paste(&CleanedText::new(DICTATED.to_owned())?)
}

#[when("the user quits the app")]
fn user_quits(world: &mut IntegrationWorld) {
    // the stopping app drops its pipeline's paster
    drop(clipboard(world).paster.take());
}

#[when("the user copies something new")]
fn copies_something_new(world: &mut IntegrationWorld) {
    clipboard(world).copy(plain_text("copied after the paste"));
}

#[then("the clipboard holds the screenshot again after the paste")]
#[then("the clipboard holds that text again after the paste")]
#[then("the clipboard is empty again after the paste")]
#[then("the clipboard holds the user's text again after the paste, not the first dictation")]
fn restored_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = clipboard(world);
    let deadline = Instant::now() + RESTORE_DEADLINE;
    while !run.restored() && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    ensure!(
        run.restored(),
        "the clipboard holds {:?}",
        read_items(&run.pasteboard)
    );
    Ok(())
}

#[then("the clipboard holds the user's text again before the app ends")]
fn restored_at_quit(world: &mut IntegrationWorld) -> Result<()> {
    let run = clipboard(world);
    ensure!(
        run.restored(),
        "the clipboard holds {:?}",
        read_items(&run.pasteboard)
    );
    Ok(())
}

#[then("the clipboard keeps the new copy after the delay")]
fn keeps_new_copy(world: &mut IntegrationWorld) -> Result<()> {
    std::thread::sleep(Duration::from_millis(3 * PENDING_DELAY_MS));
    let run = clipboard(world);
    ensure!(
        run.restored(),
        "the clipboard holds {:?}",
        read_items(&run.pasteboard)
    );
    Ok(())
}
