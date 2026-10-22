use std::time::Instant;

use crate::core::application::{CommandSpotter, KeyPresser, PipelineInput};
use crate::core::domain::{
    AudioBuffer16k, CommandPlacement, FailureNotice, SpokenCommand, SpottingEvent, VoiceCommand,
    VoiceCommandRecognizer, VoiceStartTiming,
};

use super::failure_notices::{command_failed, RecordingNotStarted};
use super::{Pipeline, StartCue};

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

impl Pipeline {
    /// Turns voice control on: listening runs alongside push-to-talk, and spoken commands
    /// drive dictations and key presses as the hotkey does.
    pub fn with_voice_control(
        mut self,
        spotter: Box<dyn CommandSpotter>,
        key_presser: Box<dyn KeyPresser>,
    ) -> Self {
        self.voice = Some(VoiceControl {
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

    pub(super) fn handle_with_voice_control(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        match &input {
            PipelineInput::Spotting(event) => log::debug!("voice control recv {event:?}"),
            other => log::info!(
                "voice control recv {other:?}, listening={}",
                self.listening()
            ),
        }
        match input {
            PipelineInput::Spotting(SpottingEvent::ConfirmationFailed { .. })
                if self.listening() =>
            {
                // A command said in the segment is lost: say so, as when the service is away.
                self.show_failure_at_once(FailureNotice::VoiceCommandsUnavailable);
                Ok(())
            }
            PipelineInput::Spotting(event) => {
                for spoken in self.recognize(event) {
                    self.carry_out_or_log(spoken);
                }
                Ok(())
            }
            PipelineInput::SpottingLost => self.lose_spotting(),
            PipelineInput::SpottingStillAway => {
                self.show_commands_unavailable();
                Ok(())
            }
            PipelineInput::SpottingBack => self.regain_spotting(),
            other => {
                log::warn!("voice control cannot handle {other:?}: the pipeline handles it");
                Ok(())
            }
        }
    }

    fn voice(&mut self) -> &mut VoiceControl {
        self.voice.as_mut().expect("voice control is on")
    }

    fn listening(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.listening)
    }

    /// Points the service at the phrases of a running dictation or of standby, whatever
    /// started or ended the dictation; nothing without voice control or while not listening.
    pub(super) fn follow_dictation(&mut self, dictating: bool) {
        let Some(voice) = self.voice.as_mut().filter(|voice| voice.listening) else {
            return;
        };
        voice.spotter.listen_for(if dictating {
            &VoiceCommand::DICTATING
        } else {
            &VoiceCommand::STANDBY
        });
    }

    /// The sound heard since `since`; none without voice control.
    pub(super) fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        self.voice.as_ref().map_or_else(
            || AudioBuffer16k::new(Vec::new()),
            |voice| voice.spotter.heard_since(since),
        )
    }

    pub(super) fn turn_listening_on(&mut self) {
        log::info!("voice control: turning listening on…");
        let voice = self.voice();
        voice.recognizer = VoiceCommandRecognizer::new();
        if let Err(e) = voice.spotter.start_listening(&VoiceCommand::STANDBY) {
            log::error!("voice control: cannot listen, command spotting service: {e:#}");
            self.show_failure_at_once(FailureNotice::VoiceCommandsUnavailable);
            return;
        }
        voice.listening = true;
        self.sound_player.play_listening_on();
        log::info!("voice control: listening");
    }

    /// Closes the microphone of voice control; nothing without voice control.
    pub(super) fn turn_listening_off(&mut self) {
        let Some(voice) = self.voice.as_mut().filter(|voice| voice.listening) else {
            return;
        };
        voice.spotter.stop_listening();
        voice.listening = false;
        log::info!("voice control: listening off");
    }

    fn recognize(&mut self, event: SpottingEvent) -> Vec<SpokenCommand> {
        if !self.listening() {
            return Vec::new();
        }
        self.voice().recognizer.feed(event)
    }

    /// A command that fails (the microphone is busy, a key press is refused) is logged and
    /// announced; the app keeps listening for the next one.
    fn carry_out_or_log(&mut self, spoken: SpokenCommand) {
        let voice = self.voice();
        if voice
            .carried_before_outage
            .is_some_and(|until| spoken.ends_at <= until)
        {
            log::info!(
                "voice control: \"{}\" was carried out before the outage, skipped",
                spoken.command.phrase()
            );
            return;
        }
        voice.last_carried_end = Some(
            voice
                .last_carried_end
                .map_or(spoken.ends_at, |end| end.max(spoken.ends_at)),
        );
        if let Err(e) = self.carry_out(spoken) {
            log::error!(
                "voice control: \"{}\" failed: {e:#}",
                spoken.command.phrase()
            );
            // a recording that did not start shows its own notice already
            if !e.is::<RecordingNotStarted>() {
                self.show_failure_at_once(command_failed(spoken.command, &e));
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn carry_out(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        let phrase = spoken.command.phrase();
        let accepted: &[VoiceCommand] = if self.is_recording() {
            &VoiceCommand::DICTATING
        } else {
            &VoiceCommand::STANDBY
        };
        if !accepted.contains(&spoken.command) {
            log::info!("voice control: \"{phrase}\" ignored in this state");
            return Ok(());
        }
        log::info!("voice control: \"{phrase}\"");
        match spoken.command {
            VoiceCommand::StartDictation => {
                self.start_dictation_after(spoken, StartCue::Start)?;
                log_voice_start(&spoken);
                Ok(())
            }
            VoiceCommand::StopDictation => {
                self.command_words.closing = spoken.placement != CommandPlacement::Alone;
                self.recorder.truncate_at(spoken.started_at);
                self.stop_recording()
            }
            VoiceCommand::PressEnter if self.is_recording() => {
                // "enter" during a dictation is "stop enter": paste, then send.
                self.command_words.closing = spoken.placement != CommandPlacement::Alone;
                self.recorder.truncate_at(spoken.started_at);
                self.stop_recording()?;
                self.press_keys(VoiceCommand::PressEnter)
            }
            VoiceCommand::CancelDictation => self.cancel_recording(),
            VoiceCommand::RestartDictation => {
                // One restart cue instead of the cancel and start cues, so it is heard apart;
                // it never waits for the dropped dictation's last request.
                self.drop_recording()?;
                self.start_dictation_after(spoken, StartCue::Restart)?;
                self.visual_indicator
                    .show_command_sign(VoiceCommand::RestartDictation);
                log_voice_start(&spoken);
                Ok(())
            }
            VoiceCommand::PressEnter | VoiceCommand::ClearAllText | VoiceCommand::PasteText => {
                self.press_keys(spoken.command)
            }
        }
    }

    /// Starts a dictation that begins where `spoken` ended, so the speech said right after
    /// the command, before it was recognized, is part of it.
    fn start_dictation_after(
        &mut self,
        spoken: SpokenCommand,
        cue: StartCue,
    ) -> anyhow::Result<()> {
        self.start_recording_from(Some(spoken.ends_at), cue)?;
        // Only an end the voice activity detector placed lies safely past the last word.
        self.command_words.opening =
            (spoken.placement != CommandPlacement::Alone).then_some(spoken.command);
        Ok(())
    }

    /// Presses the keys of "press enter", "clear all text" or "paste text", then confirms the
    /// command.
    fn press_keys(&mut self, command: VoiceCommand) -> anyhow::Result<()> {
        let keys = self.voice().key_presser.as_mut();
        let pressed = match command {
            VoiceCommand::ClearAllText => keys.clear_all_text(),
            VoiceCommand::PasteText => keys.paste(),
            _ => keys.press_enter(),
        };
        match pressed {
            Ok(()) => self.confirm(command),
            Err(e) => {
                log::error!("voice control: key press failed: {e:#}");
                self.show_failure_at_once(command_failed(command, &e));
            }
        }
        Ok(())
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
    fn lose_spotting(&mut self) -> anyhow::Result<()> {
        if !self.listening() || self.voice().away {
            return Ok(());
        }
        log::error!(
            "voice control: the command spotting service stopped answering; listening resumes once it is back"
        );
        let recording = self.is_recording();
        let voice = self.voice();
        voice.away = true;
        voice.carried_before_outage = voice.last_carried_end;
        log::info!(
            "voice control: listening → away{}",
            if recording {
                ", the dictation keeps recording"
            } else {
                ""
            }
        );
        Ok(())
    }

    /// The service stayed away: the overlay shows that voice commands do not work, once.
    fn show_commands_unavailable(&mut self) {
        if !self.listening() || !self.voice().away || self.voice().unavailable_shown {
            return;
        }
        self.voice().unavailable_shown = true;
        self.visual_indicator.show_commands_unavailable();
        log::info!("voice control: away → voice commands shown as unavailable");
    }

    /// The service is back: the commands of the current state are spotted again.
    fn regain_spotting(&mut self) -> anyhow::Result<()> {
        if !self.listening() || !self.voice().away {
            return Ok(());
        }
        let recording = self.is_recording();
        let voice = self.voice();
        voice.away = false;
        voice.recognizer = VoiceCommandRecognizer::new();
        self.follow_dictation(recording);
        if std::mem::take(&mut self.voice().unavailable_shown) {
            self.visual_indicator.hide_commands_unavailable();
        }
        log::info!(
            "voice control: away → listening again{}",
            if recording {
                " during the dictation"
            } else {
                ""
            }
        );
        Ok(())
    }
}

/// How long the start cue `spoken` just gave took, stage by stage.
fn log_voice_start(spoken: &SpokenCommand) {
    log::info!(
        "voice start: {}",
        VoiceStartTiming::measure(spoken, Instant::now())
    );
}

#[cfg(test)]
mod tests;
