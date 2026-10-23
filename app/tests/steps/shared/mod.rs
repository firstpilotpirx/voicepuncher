//! The steps both harnesses use (`SharedSteps`, crate `test-steps`) at the slice and component
//! level: each runs the step function this harness keeps for it.

use anyhow::{bail, Result};
use voicepuncher_test_steps::{shared_steps, SharedSteps};

use super::{dictation_pointer, failure_notices, permissions_gate, voice_control};
use crate::IntegrationWorld;

shared_steps!(IntegrationWorld);

#[allow(
    clippy::future_not_send,
    reason = "the slice world holds doubles that are not Send; cucumber runs steps on its local executor"
)]
impl SharedSteps for IntegrationWorld {
    async fn voice_control_is_turned_on(&mut self) -> Result<()> {
        voice_control::voice_control_on(self);
        Ok(())
    }

    async fn listening_is_on(&mut self) -> Result<()> {
        voice_control::listening_on_given(self)
    }

    async fn the_overlay_is_hidden(&mut self) -> Result<()> {
        voice_control::overlay_hidden_given(self)
    }

    async fn commands_unavailable_shown_given(&mut self) -> Result<()> {
        voice_control::sign_shown_given(self)
    }

    async fn command_spotting_went_away_and_is_back(&mut self) -> Result<()> {
        voice_control::service_went_away_and_is_back(self)
    }

    async fn the_app_starts(&mut self) -> Result<()> {
        voice_control::app_starts(self);
        Ok(())
    }

    async fn command_spotting_stays_away(&mut self) -> Result<()> {
        dictation_pointer::spotting_stays_away(self)
    }

    async fn commands_unavailable_shown(&mut self) -> Result<()> {
        bail!("the component level sets the sign up as a Given only: the E2E level checks it")
    }

    async fn no_listening_sound_after_start(&mut self) -> Result<()> {
        voice_control::no_listening_sound_after_start(self)
    }

    async fn no_permission_is_checked(&mut self) -> Result<()> {
        permissions_gate::no_permission_checked(self)
    }

    async fn only_the_other_sentence_is_pasted(&mut self) -> Result<()> {
        voice_control::only_other_sentence(self)
    }

    async fn return_pressed_after_the_paste(&mut self) -> Result<()> {
        voice_control::return_after_paste(self)
    }

    async fn return_pressed(&mut self) -> Result<()> {
        voice_control::return_pressed(self)
    }

    async fn the_raw_transcript_is_pasted(&mut self) -> Result<()> {
        failure_notices::raw_pasted(self)
    }

    async fn the_sentence_is_pasted(&mut self) -> Result<()> {
        voice_control::sentence_pasted(self)
    }
}
