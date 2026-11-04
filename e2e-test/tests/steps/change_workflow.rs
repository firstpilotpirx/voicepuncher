//! Steps of `change_workflow.feature`: what the check's own scenarios promise about their run time.

use anyhow::{bail, Context as _, Result};
use cucumber::then;

use crate::E2eWorld;

/// The session's audio outpaces real time by this factor: the room between its steps is fed ten
/// times faster, so the whole session takes well under a tenth of its audio's length.
const AUDIO_OVER_TIME: f32 = 10.0;

#[then("the session ended in less than a tenth of the time its audio lasts")]
fn session_outpaced_its_audio(world: &mut E2eWorld) -> Result<()> {
    let session = world
        .voice
        .session
        .as_ref()
        .context("listening is not on")?;
    let ran_for = session.ran_for().context("the session did not run")?;
    let audio_secs = session.audio_secs();
    if ran_for.as_secs_f32() * AUDIO_OVER_TIME >= audio_secs {
        bail!("the session ran for {ran_for:.1?}, its audio lasts {audio_secs:.1} s");
    }
    Ok(())
}
