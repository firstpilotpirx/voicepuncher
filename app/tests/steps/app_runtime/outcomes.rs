//! Then steps of the runtime's state, its pipelines and its log.

use super::{
    bail, run, then, Context, Event, IntegrationWorld, RecordingMode, Result, RuntimeRun,
    RuntimeState, FIRST_HOTKEY, SECOND_HOTKEY, SETTLE, THIRD_HOTKEY,
};

#[then("the dictation pipeline runs with that configuration")]
pub(super) fn pipeline_runs_with_configuration(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.started
        .take()
        .context("the runtime was not started")??;
    run.wait_for(|e| *e == Event::Assembled(1, FIRST_HOTKEY))?;
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!(
            "the configured hotkey started no recording: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then("the runtime state is Ready with voice control off")]
pub(super) fn ready_voice_control_off(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| {
        *s == RuntimeState::Ready {
            hotkey: FIRST_HOTKEY,
            recording_mode: RecordingMode::Toggle,
            voice_control: false,
        }
    })
}

#[then("the runtime state is Failed with a reason naming the speech recognition service")]
pub(super) fn failed_naming_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    match run.started.take().context("the runtime was not started")? {
        Ok(()) => bail!("the runtime started without the speech recognition service"),
        Err(e) if format!("{e:#}").contains("speech recognition service") => {}
        Err(e) => bail!("the start failed for another reason: {e:#}"),
    }
    run.state_becomes(
        |s| matches!(s, RuntimeState::Failed(reason) if reason.contains("speech recognition service")),
    )
}

#[then("no dictation pipeline runs")]
pub(super) fn no_pipeline(world: &mut IntegrationWorld) -> Result<()> {
    run(world).no_pipeline_runs()
}

#[then("the runtime state is Ready again")]
pub(super) fn ready_again(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == SECOND_HOTKEY),
    )
}

#[then("a press of the second hotkey starts a recording")]
pub(super) fn second_hotkey_records(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.recordings_after(SECOND_HOTKEY) != 1 {
        bail!("the second hotkey started no recording: {:?}", run.events());
    }
    // stop it, so the next press starts no recording only because of its key
    run.keyboard.press(SECOND_HOTKEY);
    run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    Ok(())
}

#[then("a press of the first hotkey starts no recording")]
pub(super) fn first_hotkey_silent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.recordings_after(FIRST_HOTKEY) != 0 {
        bail!("the first hotkey still records: {:?}", run.events());
    }
    Ok(())
}

#[then("the process is the same one")]
pub(super) fn same_process(world: &mut IntegrationWorld) -> Result<()> {
    if run(world).observed.process != std::process::id() {
        bail!("the process changed");
    }
    Ok(())
}

#[then("the pipeline runs with the second configuration")]
pub(super) fn runs_with_second(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == THIRD_HOTKEY),
    )?;
    let generation = run.latest_generation().context("no pipeline assembled")?;
    if run
        .position(|e| *e == Event::Assembled(generation, THIRD_HOTKEY))
        .is_err()
    {
        bail!(
            "the last pipeline is not the second configuration's: {:?}",
            run.events()
        );
    }
    if run.recordings_after(THIRD_HOTKEY) != 1 {
        bail!(
            "the second configuration's hotkey started no recording: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then("the runtime state is Ready")]
#[then("the runtime state stays Ready")]
pub(super) fn state_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))
}

#[then("the runtime state is Restarting")]
pub(super) fn state_restarting(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| *s == RuntimeState::Restarting)
}

#[then("the recording keeps running until the hotkey stops it")]
pub(super) fn recording_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let ended = run.count(|e| matches!(e, Event::CancelPlayed(1) | Event::PipelineDropped(1)));
    if ended > 0 || run.last_state() != Some(RuntimeState::Restarting) {
        bail!(
            "the restart did not wait for the recording: {:?}",
            run.events()
        );
    }
    run.keyboard.press(FIRST_HOTKEY);
    Ok(())
}

#[then("the dictation is pasted")]
pub(super) fn dictation_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::Pasted(1))?;
    Ok(())
}

#[then("only then the pipeline restarts and the runtime state is Ready")]
pub(super) fn restarts_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    restarted_after_the_dictation(run(world))
}

#[then("the pipeline restarts only after that dictation was pasted or given up")]
pub(super) fn restarts_after_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(|e| matches!(e, Event::PipelineDropped(1))) > 0 {
        bail!(
            "the pipeline restarted while the dictation waited: {:?}",
            run.events()
        );
    }
    *run.text_field_focused.lock().unwrap() = true;
    run.wait_for(|e| *e == Event::Pasted(1))?;
    restarted_after_the_dictation(run)
}

/// Checks the order Restarting, the old pipeline's paste, its end, Ready.
pub(crate) fn restarted_after_the_dictation(run: &RuntimeRun) -> Result<()> {
    run.wait_for(|e| *e == Event::PipelineDropped(1))?;
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let order = restart_order(run)?;
    if !order.windows(2).all(|pair| pair[0] < pair[1]) {
        bail!(
            "expected Restarting, the paste, the old pipeline's end, Ready: {:?}",
            run.events()
        );
    }
    Ok(())
}

/// Where Restarting, the paste, the old pipeline's end and the last Ready are among the events.
fn restart_order(run: &RuntimeRun) -> Result<[usize; 4]> {
    Ok([
        run.position(|e| *e == Event::State(RuntimeState::Restarting))?,
        run.position(|e| *e == Event::Pasted(1))?,
        run.position(|e| *e == Event::PipelineDropped(1))?,
        last_ready(&run.events()),
    ])
}

/// Where the last Ready is among `events`; past them when none is.
fn last_ready(events: &[Event]) -> usize {
    events
        .iter()
        .rposition(|e| matches!(e, Event::State(RuntimeState::Ready { .. })))
        .unwrap_or(events.len())
}

#[then("a press of the first hotkey still starts a recording")]
pub(super) fn first_hotkey_still_records(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!("the first hotkey no longer records: {:?}", run.events());
    }
    if run.latest_generation() != Some(1) {
        bail!("the pipeline was replaced: {:?}", run.events());
    }
    Ok(())
}

#[then("the log names the reason the configuration was rejected")]
pub(super) fn log_names_reason(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let named = run.observed.rejection_log.iter().any(|line| {
        line.contains("configuration rejected") && line.contains("transcription.base_url")
    });
    if !named {
        bail!(
            "no rejection with its reason in the log: {:?}",
            run.observed.rejection_log
        );
    }
    Ok(())
}

#[then("the pipeline ends")]
pub(super) fn pipeline_ends(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::PipelineDropped(1))?;
    Ok(())
}

#[then("voice control listening stops")]
pub(super) fn listening_stops(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ListeningOff(1))?;
    Ok(())
}

#[then("the runtime state is Stopped")]
pub(super) fn state_stopped(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| *s == RuntimeState::Stopped)
}

#[then("the recording is cancelled and nothing is pasted")]
pub(super) fn recording_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    run(world).cancelled_and_nothing_pasted()
}
