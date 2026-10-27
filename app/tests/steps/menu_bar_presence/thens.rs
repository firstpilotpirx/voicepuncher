//! Then steps: the icon, the status line and the menu's effects.

use super::{
    bail, run, then, Context, IconState, IntegrationWorld, PipelineInput, QuitEvent, Result,
    NOTICE_LIFETIME, NOTICE_SLACK, SETTLE,
};

#[then("the settings window opens in front")]
pub(super) fn settings_window_opens(world: &mut IntegrationWorld) -> Result<()> {
    let opens = *run(world).chosen.settings_opens.lock().unwrap();
    if opens != 1 {
        bail!("the settings window was opened {opens} times");
    }
    Ok(())
}

#[then("the history window opens in front")]
pub(super) fn history_window_opens(world: &mut IntegrationWorld) -> Result<()> {
    let opens = *run(world).chosen.history_opens.lock().unwrap();
    if opens != 1 {
        bail!("the history window was opened {opens} times");
    }
    Ok(())
}

#[then("the runtime stops and the process ends with exit code 0")]
pub(super) fn process_ends(world: &mut IntegrationWorld) -> Result<()> {
    let log = run(world).chosen.quit_log.lock().unwrap().clone();
    if log != [QuitEvent::RuntimeStopped, QuitEvent::ProcessEnded(0)] {
        bail!("the quit did: {log:?}");
    }
    Ok(())
}

#[then("the recording is cancelled with the cancel sound and nothing is pasted")]
pub(super) fn recording_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if !*run.parts.cancel_played.lock().unwrap() {
        bail!("no cancel sound played");
    }
    if *run.parts.pasted.lock().unwrap() {
        bail!("text was pasted");
    }
    Ok(())
}

#[then("the status item shows the waiting icon")]
pub(super) fn shows_waiting(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Waiting)
}

#[then("the status item shows the ready icon")]
pub(super) fn shows_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Ready)
}

#[then("the status item shows the waiting icon during the restart and the ready icon afterwards")]
pub(super) fn waiting_during_restart(world: &mut IntegrationWorld) -> Result<()> {
    run(world).drawn_since_becomes(&[IconState::Waiting, IconState::Ready])
}

#[then("the status item shows the failed icon and the start reports the failure the app exits on")]
pub(super) fn shows_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::Failed)?;
    match &run.started {
        Some(Err(_)) => Ok(()),
        other => bail!("the start did not fail: {other:?}"),
    }
}

#[then("the status item shows the recording icon")]
pub(super) fn shows_recording(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Recording)
}

#[then(
    "the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste"
)]
fn shows_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    run(world).drawn_since_becomes(&[
        IconState::Recording,
        IconState::Processing,
        IconState::WaitingForFocus,
        IconState::Ready,
    ])
}

#[then(
    "the status item shows the warning icon while the failure notice shows and the ready icon once it hides"
)]
fn warning_while_notice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::Warning)?;
    run.icon_is(IconState::Ready)?;
    let history = run.icon_history();
    let warned = history
        .iter()
        .rposition(|(_, icon)| *icon == IconState::Warning)
        .context("no warning drawn")?;
    let shown_for = history[warned + 1].0 - history[warned].0;
    if shown_for + NOTICE_SLACK < NOTICE_LIFETIME {
        bail!("the warning showed only {shown_for:?}");
    }
    Ok(())
}

#[then("the status item keeps showing the recording icon")]
pub(super) fn keeps_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    match run.current_icon() {
        Some(IconState::Recording) => Ok(()),
        other => bail!("the icon is {other:?}; drawn: {:?}", run.icon_history()),
    }
}

#[then("the status item shows the listening icon")]
pub(super) fn shows_listening(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Listening)
}

#[then("the status item shows the ear with a warning badge until the sign goes")]
pub(super) fn ear_with_badge(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::CommandsUnavailable)?;
    run.send(PipelineInput::SpottingBack)?;
    run.icon_is(IconState::Listening)
}

#[then("the menu's first line names the app and its version")]
pub(super) fn first_line(world: &mut IntegrationWorld) -> Result<()> {
    let menu = run(world).menu.clone().context("the menu was not opened")?;
    let expected = format!("voicepuncher {}", env!("CARGO_PKG_VERSION"));
    if menu.title_line != expected {
        bail!("the first line reads {:?}", menu.title_line);
    }
    Ok(())
}

#[then(expr = "the status line reads {string}")]
pub(super) fn status_line_reads(world: &mut IntegrationWorld, expected: String) -> Result<()> {
    let line = run(world).status_line()?;
    if line != expected {
        bail!("the status line reads {line:?}");
    }
    Ok(())
}

#[then(expr = "the status line starts with {string} and names the speech recognition service")]
pub(super) fn status_line_failed(world: &mut IntegrationWorld, prefix: String) -> Result<()> {
    let line = run(world).status_line()?;
    if !line.starts_with(&prefix) || !line.contains("speech recognition service") {
        bail!("the status line reads {line:?}");
    }
    Ok(())
}
