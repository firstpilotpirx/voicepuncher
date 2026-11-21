//! The Claude desktop app's own dictation, driven by its shortcut and Escape, posted as a
//! keyboard does; Claude is told apart by the frontmost app's bundle id.

use anyhow::Result;
use core_graphics::event::{CGEventFlags, CGKeyCode, KeyCode};
use objc2_app_kit::NSWorkspace;

use crate::adapters::secondary::keypressing::macos::keystroke;
use crate::adapters::secondary::permissions::macos::process_is_trusted_for_accessibility;
use crate::core::application::ClaudeDictation;
use crate::core::domain::{AccessibilityMissing, KeyChord};

/// The bundle id of the Claude desktop app, where Claude Code runs in its Code tab.
pub const CLAUDE_BUNDLE_ID: &str = "com.anthropic.claudefordesktop";

type Press = (CGKeyCode, CGEventFlags);

const ESCAPE: Press = (KeyCode::ESCAPE, CGEventFlags::CGEventFlagNull);

/// Presses Claude's dictation shortcut and Escape as `MacOsKeyPresser` presses its keys.
pub struct MacOsClaudeDictation {
    shortcut: Press,
    /// Whether macOS trusts the process for accessibility; without it key presses are dropped.
    trusted: fn() -> bool,
    /// The bundle id of the frontmost app, if it has one.
    frontmost: fn() -> Option<String>,
}

impl MacOsClaudeDictation {
    pub fn new(shortcut: KeyChord) -> Self {
        Self {
            shortcut: (key_code(shortcut.key()), CGEventFlags::CGEventFlagCommand),
            trusted: process_is_trusted_for_accessibility,
            frontmost: frontmost_bundle_id,
        }
    }

    fn press(&self, (key, flags): Press) -> Result<()> {
        if !(self.trusted)() {
            return Err(AccessibilityMissing.into());
        }
        // pressed for the user, as the user's own keys
        keystroke::press(key, flags, keystroke::Level::Keyboard)
    }
}

impl ClaudeDictation for MacOsClaudeDictation {
    fn is_frontmost(&self) -> bool {
        (self.frontmost)().as_deref() == Some(CLAUDE_BUNDLE_ID)
    }

    fn toggle(&mut self) -> Result<()> {
        self.press(self.shortcut)
    }

    fn cancel(&mut self) -> Result<()> {
        self.press(ESCAPE)
    }
}

fn frontmost_bundle_id() -> Option<String> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    app.bundleIdentifier().map(|id| id.to_string())
}

/// The ANSI key codes of the letters a to z.
const LETTERS: [CGKeyCode; 26] = [
    KeyCode::ANSI_A,
    KeyCode::ANSI_B,
    KeyCode::ANSI_C,
    KeyCode::ANSI_D,
    KeyCode::ANSI_E,
    KeyCode::ANSI_F,
    KeyCode::ANSI_G,
    KeyCode::ANSI_H,
    KeyCode::ANSI_I,
    KeyCode::ANSI_J,
    KeyCode::ANSI_K,
    KeyCode::ANSI_L,
    KeyCode::ANSI_M,
    KeyCode::ANSI_N,
    KeyCode::ANSI_O,
    KeyCode::ANSI_P,
    KeyCode::ANSI_Q,
    KeyCode::ANSI_R,
    KeyCode::ANSI_S,
    KeyCode::ANSI_T,
    KeyCode::ANSI_U,
    KeyCode::ANSI_V,
    KeyCode::ANSI_W,
    KeyCode::ANSI_X,
    KeyCode::ANSI_Y,
    KeyCode::ANSI_Z,
];
/// The ANSI key codes of the digits 0 to 9.
const DIGITS: [CGKeyCode; 10] = [
    KeyCode::ANSI_0,
    KeyCode::ANSI_1,
    KeyCode::ANSI_2,
    KeyCode::ANSI_3,
    KeyCode::ANSI_4,
    KeyCode::ANSI_5,
    KeyCode::ANSI_6,
    KeyCode::ANSI_7,
    KeyCode::ANSI_8,
    KeyCode::ANSI_9,
];

/// The key code of a lowercase letter or digit on the ANSI layout.
fn key_code(key: char) -> CGKeyCode {
    let index = |first: char| usize::from(key as u8 - first as u8);
    match key {
        'a'..='z' => LETTERS[index('a')],
        '0'..='9' => DIGITS[index('0')],
        _ => KeyCode::ANSI_D,
    }
}

#[cfg(test)]
mod tests;
