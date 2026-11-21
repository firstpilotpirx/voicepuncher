//! A Claude dictation: in the Claude desktop app, the pipeline presses Claude's own
//! dictation shortcut instead of recording, with the cues and overlay of its own dictation.

use std::time::{Duration, Instant};

use crate::core::application::ClaudeDictation;
use crate::core::domain::SpokenCommand;

use super::voice_control::log_voice_start;
use super::{Pipeline, StartCue};

/// How long Claude takes to finish recognizing and insert its text after its dictation stops.
pub const CLAUDE_TEXT_WAIT: Duration = Duration::from_millis(1500);

/// The port, and whether a Claude dictation runs now.
pub(super) struct ClaudeDictationState {
    port: Box<dyn ClaudeDictation>,
    running: bool,
    text_wait: Duration,
}

impl Pipeline {
    /// In the Claude desktop app, a dictation drives Claude's own dictation through `port`.
    pub fn with_claude_dictation(mut self, port: Box<dyn ClaudeDictation>) -> Self {
        self.claude =
            Some(ClaudeDictationState { port, running: false, text_wait: CLAUDE_TEXT_WAIT });
        self
    }

    /// Claude's text is waited for `wait` before "press enter" presses Return.
    pub fn with_claude_text_wait(mut self, wait: Duration) -> Self {
        if let Some(claude) = self.claude.as_mut() {
            claude.text_wait = wait;
        }
        self
    }

    pub(super) fn claude_dictation_runs(&self) -> bool {
        self.claude.as_ref().is_some_and(|claude| claude.running)
    }

    /// Starts Claude's dictation when Claude is the frontmost app: the shortcut first, then
    /// the cue and the overlay. False when the app's own dictation must start instead.
    pub(super) fn start_claude_dictation(
        &mut self,
        cue: StartCue,
        voice_start: Option<SpokenCommand>,
    ) -> bool {
        let Some(claude) = self.claude.as_mut() else {
            return false;
        };
        if !claude.port.is_frontmost() {
            return false;
        }
        if let Err(e) = claude.port.toggle() {
            log::warn!("claude dictation: not used: {e:#}");
            return false;
        }
        claude.running = true;
        log::info!("claude dictation: started");
        self.voice.follow_dictation(true);
        self.play_start_cue(cue, voice_start);
        true
    }

    /// The start cue and the recording overlay; a voice start logs the time of its cue.
    pub(super) fn play_start_cue(&self, cue: StartCue, voice_start: Option<SpokenCommand>) {
        match cue {
            StartCue::Start => self.sound_player.play_start(),
            StartCue::Restart => self.sound_player.play_restart(),
        }
        let cue_at = Instant::now();
        match voice_start {
            Some(spoken) => self
                .visual_indicator
                .show_recording_then(Box::new(move |drawn_at| {
                    log_voice_start(&spoken, cue_at, drawn_at);
                })),
            None => self.visual_indicator.show_recording(),
        }
    }

    /// Stops Claude's dictation: Claude inserts its text itself, so nothing is left to show.
    pub(super) fn stop_claude_dictation(&mut self) {
        self.voice.follow_dictation(false);
        if let Some(claude) = self.claude.as_mut() {
            claude.running = false;
            if let Err(e) = claude.port.toggle() {
                log::error!("claude dictation: not stopped: {e:#}");
            }
        }
        log::info!("claude dictation: stopped");
        self.sound_player.play_stop();
        self.visual_indicator.hide();
    }

    /// Drops Claude's dictation, pressing Escape unless the user's own Escape reached Claude.
    pub(super) fn drop_claude_dictation(&mut self, press_escape: bool) {
        self.voice.follow_dictation(false);
        let Some(claude) = self.claude.as_mut() else {
            return;
        };
        claude.running = false;
        if press_escape {
            if let Err(e) = claude.port.cancel() {
                log::error!("claude dictation: not dropped: {e:#}");
            }
        }
        log::info!("claude dictation: dropped");
    }

    /// Waits for Claude to insert the text of the dictation just stopped.
    pub(super) fn wait_for_claude_text(&self) {
        if let Some(claude) = self.claude.as_ref() {
            std::thread::sleep(claude.text_wait);
        }
    }
}
