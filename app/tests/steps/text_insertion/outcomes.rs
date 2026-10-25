//! Then steps: when and where the text is pasted.

use super::{
    bail, is_left_in_clipboard, is_pasted, run, then, Context, Duration, Event, Instant,
    IntegrationWorld, Result, DICTATED, REACTION, SETTLE,
};

#[then("the text is pasted at once")]
pub(super) fn pasted_at_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(is_pasted, REACTION)?;
    if run.count(|e| *e == Event::WaitingShown) > 0 {
        bail!("the app waited for focus first: {:?}", run.events());
    }
    Ok(())
}

#[then("the overlay never shows that it waits for focus")]
pub(super) fn never_waited(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(|e| *e == Event::WaitingShown) > 0 {
        bail!("the overlay showed the wait: {:?}", run.events());
    }
    Ok(())
}

#[then("nothing is pasted yet")]
#[then("nothing is pasted")]
pub(super) fn nothing_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(is_pasted) > 0 {
        bail!("text was pasted: {:?}", run.events());
    }
    Ok(())
}

#[then("the overlay shows that the app waits for focus")]
pub(super) fn waiting_shown(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[then("the text is pasted into it")]
pub(super) fn pasted_after_wait(world: &mut IntegrationWorld) -> Result<()> {
    let event = run(world).wait_for(is_pasted, REACTION)?;
    if event != Event::Pasted(DICTATED.to_owned()) {
        bail!("pasted something else: {event:?}");
    }
    Ok(())
}

#[then("the overlay hides")]
pub(super) fn overlay_hides(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let outcome = run.wait_for(|e| is_pasted(e) || is_left_in_clipboard(e), REACTION)?;
    let deadline = Instant::now() + REACTION;
    loop {
        let events = run.events();
        let delivered = events.iter().position(|e| *e == outcome).unwrap();
        if events[delivered..].contains(&Event::Hidden) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("overlay not hidden after {outcome:?}: {events:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then("the clipboard holds the text")]
pub(super) fn clipboard_holds_text(world: &mut IntegrationWorld) -> Result<()> {
    let event = run(world).wait_for(is_left_in_clipboard, REACTION)?;
    if event != Event::LeftInClipboard(DICTATED.to_owned()) {
        bail!("clipboard holds something else: {event:?}");
    }
    Ok(())
}

#[then("no recording starts")]
pub(super) fn no_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let started = run.count(|e| *e == Event::RecordingStarted);
    if started != 1 {
        bail!("{started} recordings started, expected only the dictation's");
    }
    let waiting_at = run
        .waiting_shown_at
        .lock()
        .unwrap()
        .context("the app did not wait for focus")?;
    let left_at = run
        .left_in_clipboard_at
        .lock()
        .unwrap()
        .context("the text was not left in the clipboard")?;
    if left_at.duration_since(waiting_at) >= run.wait_time {
        bail!("the wait ended by its time, not by the hotkey press");
    }
    Ok(())
}

#[then("a recording starts")]
pub(super) fn recording_starts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let deadline = Instant::now() + REACTION;
    while run.count(|e| *e == Event::RecordingStarted) < 2 {
        if Instant::now() >= deadline {
            bail!("no new recording: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
