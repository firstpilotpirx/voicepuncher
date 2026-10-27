use std::time::Instant;

use crate::core::application::{CommandSpotter, KeyPresser, PipelineInput};
use crate::core::domain::{
    AudioBuffer16k, CommandPlacement, FailureNotice, SpokenCommand, SpottingEvent, VoiceCommand,
    VoiceCommandRecognizer, VoiceStartTiming,
};

use super::failure_notices::{command_failed, RecordingNotStarted};
use super::recording::Awaits;
use super::{Pipeline, StartCue};

/// Voice control: off, or on with its service and the keys it presses.
pub(super) enum Voice {
    Off,
    On(VoiceControl),
}

pub(super) struct VoiceControl {
    spotter: Box<dyn CommandSpotter>,
    key_presser: Box<dyn KeyPresser>,
    recognizer: VoiceCommandRecognizer,
    listening: bool,
    /// The command spotting service is away: listening resumes once it is back.
    away: bool,
    /// The overlay shows that voice commands do not work: the service stayed away.
    unavailable_shown: bool,
    /// Where the last command carried out ended.
    last_carried_end: Option<Instant>,
    /// Commands ending by then were carried out before the service went away: the audio
    /// sent again to the service that is back may report them a second time.
    carried_before_outage: Option<Instant>,
}

impl Voice {
    pub(super) fn listening(&self) -> bool {
        matches!(self, Self::On(voice) if voice.listening)
    }

    /// Voice control while it listens; none when it is off or not listening.
    fn listening_mut(&mut self) -> Option<&mut VoiceControl> {
        match self {
            Self::On(voice) if voice.listening => Some(voice),
            _ => None,
        }
    }

    /// Points the service at the phrases of a running dictation or of standby, whatever
    /// started or ended the dictation; nothing without voice control or while not listening.
    pub(super) fn follow_dictation(&mut self, dictating: bool) {
        if let Some(voice) = self.listening_mut() {
            voice.spotter.listen_for(if dictating {
                &VoiceCommand::DICTATING
            } else {
                &VoiceCommand::STANDBY
            });
        }
    }

    /// The sound heard since `since`; none without voice control.
    pub(super) fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        match self {
            Self::On(voice) => voice.spotter.heard_since(since),
            Self::Off => AudioBuffer16k::new(Vec::new()),
        }
    }

    /// Closes the microphone of voice control; nothing without voice control.
    pub(super) fn turn_listening_off(&mut self) {
        if let Some(voice) = self.listening_mut() {
            voice.spotter.stop_listening();
            voice.listening = false;
            log::info!("voice control: listening off");
        }
    }
}

impl Pipeline {
    /// Turns voice control on: listening runs alongside push-to-talk, and spoken commands
    /// drive dictations and key presses as the hotkey does.
    pub fn with_voice_control(
        mut self,
        spotter: Box<dyn CommandSpotter>,
        key_presser: Box<dyn KeyPresser>,
    ) -> Self {
        self.voice = Voice::On(VoiceControl {
            spotter,
            key_presser,
            recognizer: VoiceCommandRecognizer::new(),
            listening: false,
            away: false,
            unavailable_shown: false,
            last_carried_end: None,
            carried_before_outage: None,
        });
        self
    }

    pub(super) fn handle_with_voice_control(&mut self, input: PipelineInput) {
        self.log_received(&input);
        match input {
            PipelineInput::Spotting(SpottingEvent::ConfirmationFailed { .. })
                if self.voice.listening() =>
            {
                // A command said in the segment is lost: say so, as when the service is away.
                self.show_failure_at_once(&FailureNotice::VoiceCommandsUnavailable);
            }
            PipelineInput::Spotting(event) => {
                for spoken in self.recognize(event) {
                    self.carry_out_or_log(spoken);
                }
            }
            PipelineInput::SpottingLost => self.lose_spotting(),
            PipelineInput::SpottingStillAway => self.show_commands_unavailable(),
            PipelineInput::SpottingBack => self.regain_spotting(),
            other => log::warn!("voice control cannot handle {other:?}: the pipeline handles it"),
        }
    }

    fn log_received(&self, input: &PipelineInput) {
        match input {
            PipelineInput::Spotting(event) => log::debug!("voice control recv {event:?}"),
            other => log::info!(
                "voice control recv {other:?}, listening={}",
                self.voice.listening()
            ),
        }
    }

    /// Opens the microphone of voice control; nothing without voice control.
    pub(super) fn turn_listening_on(&mut self) {
        let Voice::On(voice) = &mut self.voice else {
            return;
        };
        log::info!("voice control: turning listening on…");
        voice.recognizer = VoiceCommandRecognizer::new();
        if let Err(e) = voice.spotter.start_listening(&VoiceCommand::STANDBY) {
            log::error!("voice control: cannot listen, command spotting service: {e:#}");
            self.show_failure_at_once(&FailureNotice::VoiceCommandsUnavailable);
            return;
        }
        voice.listening = true;
        self.sound_player.play_listening_on();
        log::info!("voice control: listening");
    }

    fn recognize(&mut self, event: SpottingEvent) -> Vec<SpokenCommand> {
        match self.voice.listening_mut() {
            Some(voice) => voice.recognizer.feed(event),
            None => Vec::new(),
        }
    }

    /// A command that fails (the microphone is busy, a key press is refused) is logged and
    /// announced; the app keeps listening for the next one.
    fn carry_out_or_log(&mut self, spoken: SpokenCommand) {
        let phrase = spoken.command.phrase();
        let Voice::On(voice) = &mut self.voice else {
            return;
        };
        if voice
            .carried_before_outage
            .is_some_and(|until| spoken.ends_at <= until)
        {
            log::info!("voice control: \"{phrase}\" was carried out before the outage, skipped");
            return;
        }
        let end = voice
            .last_carried_end
            .map_or(spoken.ends_at, |end| end.max(spoken.ends_at));
        voice.last_carried_end = Some(end);
        if let Err(e) = self.carry_out(spoken) {
            log::error!("voice control: \"{phrase}\" failed: {e:#}");
            // a recording that did not start shows its own notice already
            if !e.is::<RecordingNotStarted>() {
                self.show_failure_at_once(&command_failed(spoken.command, &e));
            }
        }
    }

    fn carry_out(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        let phrase = spoken.command.phrase();
        if !self.accepts(spoken.command) {
            log::info!("voice control: \"{phrase}\" ignored in this state");
            return Ok(());
        }
        log::info!("voice control: \"{phrase}\"");
        self.carry_out_accepted(spoken)
    }

    fn carry_out_accepted(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        match spoken.command {
            VoiceCommand::StartDictation => self.start_dictation_after(spoken, StartCue::Start),
            VoiceCommand::RestartDictation => self.restart_dictation(spoken),
            VoiceCommand::StopDictation => self.close_dictation(spoken),
            VoiceCommand::PressEnter if self.is_recording() => {
                // "enter" during a dictation is "stop enter": paste, then send.
                self.close_dictation(spoken)?;
                self.press_keys(VoiceCommand::PressEnter);
                Ok(())
            }
            VoiceCommand::CancelDictation => self.cancel_recording(),
            VoiceCommand::PressEnter | VoiceCommand::ClearAllText | VoiceCommand::PasteText => {
                self.press_keys(spoken.command);
                Ok(())
            }
        }
    }

    /// Whether `command` is one of the commands of the current state.
    fn accepts(&self, command: VoiceCommand) -> bool {
        let accepted: &[VoiceCommand] = if self.is_recording() {
            &VoiceCommand::DICTATING
        } else {
            &VoiceCommand::STANDBY
        };
        accepted.contains(&command)
    }

    /// One restart cue instead of the cancel and start cues, so it is heard apart; it never
    /// waits for the dropped dictation's last request.
    fn restart_dictation(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        self.abandon(Awaits::Nothing)?;
        self.start_dictation_after(spoken, StartCue::Restart)?;
        self.visual_indicator
            .show_command_sign(VoiceCommand::RestartDictation);
        Ok(())
    }

    /// Starts a dictation that begins where `spoken` ended, so the speech said right after
    /// the command, before it was recognized, is part of it.
    fn start_dictation_after(
        &mut self,
        spoken: SpokenCommand,
        cue: StartCue,
    ) -> anyhow::Result<()> {
        self.start_recording_from(Some(spoken.ends_at), cue, Some(spoken))?;
        // Only an end the voice activity detector placed lies safely past the last word.
        if let Some(dictation) = self.dictation.as_mut() {
            dictation.command_words.opening =
                (spoken.placement != CommandPlacement::Alone).then_some(spoken.command);
        }
        Ok(())
    }

    /// Ends the running dictation where `spoken` began: the command's words are no part of
    /// it, unless it was said in one breath with the last words.
    fn close_dictation(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        if let Some(dictation) = self.dictation.as_mut() {
            dictation.command_words.closing = spoken.placement != CommandPlacement::Alone;
        }
        self.recorder.truncate_at(spoken.started_at);
        self.stop_recording()
    }

    /// Presses the keys of "press enter", "clear all text" or "paste text", then confirms the
    /// command.
    fn press_keys(&mut self, command: VoiceCommand) {
        let Voice::On(voice) = &mut self.voice else {
            return;
        };
        let keys = voice.key_presser.as_mut();
        let pressed = match command {
            VoiceCommand::ClearAllText => keys.clear_all_text(),
            VoiceCommand::PasteText => keys.paste(),
            _ => keys.press_enter(),
        };
        match pressed {
            Ok(()) => self.confirm(command),
            Err(e) => {
                log::error!("voice control: key press failed: {e:#}");
                self.show_failure_at_once(&command_failed(command, &e));
            }
        }
    }

    /// The sound and the command sign of a command with no visible effect of its own.
    fn confirm(&self, command: VoiceCommand) {
        match command {
            VoiceCommand::PressEnter => self.sound_player.play_enter(),
            VoiceCommand::ClearAllText => self.sound_player.play_clear(),
            VoiceCommand::PasteText => self.sound_player.play_paste(),
            _ => return,
        }
        self.visual_indicator.show_command_sign(command);
    }

    /// The service went away: listening stays on, a running dictation keeps recording, and
    /// the adapter reaches for the service until it is back.
    fn lose_spotting(&mut self) {
        let recording = self.is_recording();
        let Some(voice) = self.voice.listening_mut() else {
            return;
        };
        if voice.away {
            return;
        }
        log::error!(
            "voice control: the command spotting service stopped answering; listening resumes once it is back"
        );
        voice.away = true;
        voice.carried_before_outage = voice.last_carried_end;
        let dictation = if recording {
            ", the dictation keeps recording"
        } else {
            ""
        };
        log::info!("voice control: listening → away{dictation}");
    }

    /// The service stayed away: the overlay shows that voice commands do not work, once.
    fn show_commands_unavailable(&mut self) {
        let Some(voice) = self.voice.listening_mut() else {
            return;
        };
        if !voice.away || voice.unavailable_shown {
            return;
        }
        voice.unavailable_shown = true;
        self.visual_indicator.show_commands_unavailable();
        log::info!("voice control: away → voice commands shown as unavailable");
    }

    /// The service is back: the commands of the current state are spotted again.
    fn regain_spotting(&mut self) {
        let recording = self.is_recording();
        let Some(voice) = self.voice.listening_mut() else {
            return;
        };
        if !voice.away {
            return;
        }
        voice.away = false;
        voice.recognizer = VoiceCommandRecognizer::new();
        let unavailable_shown = std::mem::take(&mut voice.unavailable_shown);
        self.voice.follow_dictation(recording);
        if unavailable_shown {
            self.visual_indicator.hide_commands_unavailable();
        }
        let dictation = if recording {
            " during the dictation"
        } else {
            ""
        };
        log::info!("voice control: away → listening again{dictation}");
    }
}

/// How long the start cue `spoken` gave at `cue_at` took, stage by stage, its overlay drawn
/// at `drawn_at`.
pub(super) fn log_voice_start(spoken: &SpokenCommand, cue_at: Instant, drawn_at: Instant) {
    log::info!(
        "voice start: {}",
        VoiceStartTiming::measure(spoken, cue_at, Some(drawn_at))
    );
}

#[cfg(test)]
mod tests;
