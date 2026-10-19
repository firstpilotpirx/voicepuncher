//! The general pasteboard through AppKit: the text a paste puts there, and a snapshot of every item with
//! every type's data, so whatever the user had copied (text, an image, files) is put back after the paste.

use anyhow::{bail, Result};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

/// The uniform type identifier of plain UTF-8 text (`NSPasteboardTypeString`).
const PLAIN_TEXT_TYPE: &str = "public.utf8-plain-text";

/// One pasteboard item: each of its types with that type's data.
type ItemData = Vec<(String, Vec<u8>)>;

/// The change count of the general pasteboard: it moves on every write, by this process or another.
pub(super) type ChangeCount = isize;

pub(super) fn change_count() -> ChangeCount {
    NSPasteboard::generalPasteboard().changeCount()
}

/// Makes `text` the pasteboard's only content and returns the change count this write left.
pub(super) fn write_text(text: &str) -> Result<ChangeCount> {
    let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    if !pasteboard.setString_forType(
        &NSString::from_str(text),
        &NSString::from_str(PLAIN_TEXT_TYPE),
    ) {
        bail!("the pasteboard refused the text to paste");
    }
    Ok(pasteboard.changeCount())
}

/// Every item of the general pasteboard with every type's data, copied out of AppKit so it can cross threads.
pub(super) struct PasteboardSnapshot {
    items: Vec<ItemData>,
}

impl PasteboardSnapshot {
    pub(super) fn take() -> Self {
        let items: Vec<ItemData> = NSPasteboard::generalPasteboard()
            .pasteboardItems()
            .map(|items: Retained<NSArray<NSPasteboardItem>>| {
                items.iter().map(|item| read_item(&item)).collect()
            })
            .unwrap_or_default();
        Self {
            items: items
                .into_iter()
                .filter(|item: &ItemData| !item.is_empty())
                .collect(),
        }
    }

    /// Puts the items back as they were, or leaves the pasteboard empty when it was.
    pub(super) fn restore(&self) -> Result<()> {
        let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        if self.items.is_empty() {
            return Ok(());
        }
        let written: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = self
            .items
            .iter()
            .map(|item: &ItemData| ProtocolObject::from_retained(build_item(item)))
            .collect();
        if !pasteboard.writeObjects(&NSArray::from_retained_slice(&written)) {
            bail!("the pasteboard refused the items to restore");
        }
        Ok(())
    }
}

fn read_item(item: &NSPasteboardItem) -> ItemData {
    item.types()
        .iter()
        .filter_map(|data_type: Retained<NSString>| {
            let data: Retained<NSData> = item.dataForType(&data_type)?;
            Some((data_type.to_string(), data.to_vec()))
        })
        .collect()
}

fn build_item(types: &ItemData) -> Retained<NSPasteboardItem> {
    let item: Retained<NSPasteboardItem> = NSPasteboardItem::new();
    for (data_type, bytes) in types {
        item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(data_type));
    }
    item
}
