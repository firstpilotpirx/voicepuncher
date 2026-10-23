use std::time::Duration;

use anyhow::Result;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::adapters::secondary::permissions::macos::process_is_trusted_for_accessibility;
use crate::core::application::text_paster::TextPaster;
use crate::core::domain::{AccessibilityMissing, CleanedText};

mod pasteboard;
mod restore;

use pasteboard::{Board, ChangeCount, PasteboardSnapshot};
use restore::PendingRestore;

/// The time the pasteboard gets to publish the text before Cmd+V reads it.
const PASTEBOARD_SETTLE_DELAY: Duration = Duration::from_millis(20);

pub struct MacOsTextPaster {
    restore_delay_ms: u64,
    /// Whether macOS trusts the process for accessibility; without it Cmd+V is dropped.
    trusted: fn() -> bool,
    /// Sends the paste keystroke to the focused app.
    press_paste: fn() -> Result<()>,
    /// The restore the last paste left waiting; finished by the next paste and when the paster drops.
    pending: Option<PendingRestore>,
    /// The pasteboard the paste writes: the general one, except in tests.
    board: Board,
}

impl MacOsTextPaster {
    pub fn new(restore_delay_ms: u64) -> Self {
        Self {
            restore_delay_ms,
            trusted: process_is_trusted_for_accessibility,
            press_paste: Self::post_cmd_v,
            pending: None,
            board: Board::default(),
        }
    }

    /// A paster that sends `press_paste` instead of Cmd+V and needs no accessibility trust: for tests of
    /// what a paste does to the clipboard.
    pub fn with_paste_keystroke(restore_delay_ms: u64, press_paste: fn() -> Result<()>) -> Self {
        Self {
            restore_delay_ms,
            trusted: || true,
            press_paste,
            pending: None,
            board: Board::default(),
        }
    }

    /// The same paster on the pasteboard named `name` instead of the general one: for tests that must not
    /// touch the user's clipboard.
    #[must_use]
    pub fn with_pasteboard(self, name: &str) -> Self {
        Self { board: Board::named(name), ..self }
    }

    fn ensure_trusted(&self) -> Result<()> {
        if !(self.trusted)() {
            return Err(AccessibilityMissing.into());
        }
        Ok(())
    }

    fn post_cmd_v() -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|()| anyhow::anyhow!("failed to create CGEventSource"))?;

        let down = CGEvent::new_keyboard_event(source.clone(), KeyCode::ANSI_V, true)
            .map_err(|()| anyhow::anyhow!("failed to create Cmd+V key-down event"))?;
        down.set_flags(CGEventFlags::CGEventFlagCommand);
        down.post(CGEventTapLocation::Session);

        let up = CGEvent::new_keyboard_event(source, KeyCode::ANSI_V, false)
            .map_err(|()| anyhow::anyhow!("failed to create Cmd+V key-up event"))?;
        up.set_flags(CGEventFlags::CGEventFlagCommand);
        up.post(CGEventTapLocation::Session);

        Ok(())
    }
}

/// Puts the user's clipboard back, unless the pasteboard changed since the paste wrote its text: then the
/// user (or an app) copied something new, which must stay.
fn restore_unless_copied_over(
    user_clipboard: &PasteboardSnapshot,
    pasted_change_count: ChangeCount,
) {
    if user_clipboard.board().change_count() != pasted_change_count {
        log::info!("clipboard changed after the paste; not restored");
        return;
    }
    if let Err(error) = user_clipboard.restore() {
        log::warn!("clipboard restore failed: {error:#}");
    }
}

impl TextPaster for MacOsTextPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.ensure_trusted()?;
        // The previous paste's restore runs first, so this snapshot holds the user's clipboard.
        drop(self.pending.take());
        let user_clipboard = PasteboardSnapshot::take(&self.board);
        let pasted_change_count: ChangeCount = pasteboard::write_text(&self.board, text.as_str())?;
        std::thread::sleep(PASTEBOARD_SETTLE_DELAY);

        if let Err(error) = (self.press_paste)() {
            restore_unless_copied_over(&user_clipboard, pasted_change_count);
            return Err(error);
        }

        let restore_delay = Duration::from_millis(self.restore_delay_ms);
        self.pending = Some(PendingRestore::after(restore_delay, move || {
            restore_unless_copied_over(&user_clipboard, pasted_change_count);
        }));

        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        arboard::Clipboard::new()?.set_text(text.as_str().to_owned())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_fail_with_accessibility_missing_when_the_process_is_not_trusted() {
        // Given
        let mut paster = MacOsTextPaster::with_paste_keystroke(0, || Ok(()));
        paster.trusted = || false;
        let text = CleanedText::new("hello".to_owned()).unwrap();
        // When
        let error = paster.paste(&text).unwrap_err();
        // Then
        assert!(error.is::<AccessibilityMissing>(), "{error:#}");
    }
}
