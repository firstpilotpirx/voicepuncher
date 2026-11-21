//! `[claude_dictation]`: whether voicepuncher drives the Claude desktop app's own dictation.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::core::domain::KeyChord;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClaudeDictationCfg {
    /// In Claude, a dictation start presses Claude's own dictation shortcut instead of recording.
    pub enabled: bool,
    /// Claude's dictation shortcut, `cmd+` and one letter or digit.
    pub shortcut: String,
}

impl Default for ClaudeDictationCfg {
    fn default() -> Self {
        Self { enabled: true, shortcut: "cmd+d".to_owned() }
    }
}

impl ClaudeDictationCfg {
    /// The shortcut to drive Claude's dictation with; none when it is turned off.
    pub fn port_shortcut(&self) -> Result<Option<KeyChord>> {
        if self.enabled {
            self.key_chord().map(Some)
        } else {
            Ok(None)
        }
    }

    /// The shortcut as a `KeyChord`, or an error naming `claude_dictation.shortcut`.
    pub fn key_chord(&self) -> Result<KeyChord> {
        KeyChord::parse(&self.shortcut)
            .map_err(|error| anyhow!("claude_dictation.shortcut: {error}"))
    }
}

#[cfg(test)]
mod tests;
