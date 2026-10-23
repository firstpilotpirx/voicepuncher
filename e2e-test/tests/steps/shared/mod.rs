//! The steps both harnesses use (`SharedSteps`, crate `test-steps`) at the E2E level: each runs
//! the step function this harness keeps for it, on the real app and services.

use anyhow::{bail, Result};
use voicepuncher_test_steps::{shared_steps, SharedSteps};

use super::voice_control::{self, hands_free, outages, session::speech_after_start};
use super::{file_transcription, text_refinement};
use crate::E2eWorld;

shared_steps!(E2eWorld);

impl SharedSteps for E2eWorld {
    async fn voice_control_is_turned_on(&mut self) -> Result<()> {
        voice_control::voice_control_on(self);
        Ok(())
    }

    async fn listening_is_on(&mut self) -> Result<()> {
        hands_free::listening_on(self)
    }

    async fn the_overlay_is_hidden(&mut self) -> Result<()> {
        outages::overlay_hidden(self)
    }

    async fn commands_unavailable_shown_given(&mut self) -> Result<()> {
        bail!("the E2E level reaches the sign through a real outage: it checks it as a Then only")
    }

    async fn command_spotting_went_away_and_is_back(&mut self) -> Result<()> {
        outages::command_spotting_restarted(self).await
    }

    async fn the_app_starts(&mut self) -> Result<()> {
        voice_control::app_starts(self).await
    }

    async fn command_spotting_stays_away(&mut self) -> Result<()> {
        outages::command_spotting_stays_away(self).await
    }

    async fn commands_unavailable_shown(&mut self) -> Result<()> {
        outages::sign_shown(self)
    }

    async fn no_listening_sound_after_start(&mut self) -> Result<()> {
        outages::no_listening_sound_after_start(self)
    }

    async fn no_permission_is_checked(&mut self) -> Result<()> {
        file_transcription::no_permission_checked(self)
    }

    async fn only_the_other_sentence_is_pasted(&mut self) -> Result<()> {
        speech_after_start::only_other_sentence(self)
    }

    async fn return_pressed_after_the_paste(&mut self) -> Result<()> {
        hands_free::return_after_paste(self)
    }

    async fn return_pressed(&mut self) -> Result<()> {
        outages::return_pressed(self)
    }

    async fn the_raw_transcript_is_pasted(&mut self) -> Result<()> {
        text_refinement::raw_pasted(self)
    }

    async fn the_sentence_is_pasted(&mut self) -> Result<()> {
        hands_free::sentence_pasted(self)
    }
}
