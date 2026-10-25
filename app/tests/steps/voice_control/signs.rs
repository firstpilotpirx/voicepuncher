//! Steps of the overlay signs and sounds of voice commands.

use super::{
    bail, chain, run, sound_of, then, when, Duration, Event, Instant, IntegrationWorld, Result,
    VoiceRun, COMMAND_SIGNS, PASTED_SIGN, REACTION, SETTLE,
};

/// The overlay events before the step's action.
pub(crate) fn overlay_before_mark(run: &VoiceRun) -> Vec<&'static str> {
    run.events()[..run.mark]
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(*shown),
            _ => None,
        })
        .collect()
}

/// The overlay events since the step's action.
pub(crate) fn overlay_since_mark(run: &VoiceRun) -> Vec<&'static str> {
    run.since_mark()
        .into_iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(shown),
            _ => None,
        })
        .collect()
}

/// Waits for `sign` and checks nothing replaced it: the overlay takes it down on its own after
/// about a second, back to what it showed before.
pub(crate) fn ends_on_sign(run: &VoiceRun, sign: &'static str) -> Result<Vec<&'static str>> {
    run.wait_since_mark(|e| *e == Event::Overlay(sign))?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    if shown.last() != Some(&sign) {
        bail!("expected the overlay to end on the {sign}: {shown:?}");
    }
    Ok(shown)
}

#[then(expr = "the overlay shows the {word} sign for about a second, then hides")]
pub(super) fn sign_then_hides(world: &mut IntegrationWorld, command: String) -> Result<()> {
    ends_on_sign(run(world), sign_of(&command)?)?;
    Ok(())
}

#[then("the overlay shows the restart sign, then the recording")]
pub(super) fn restart_sign_then_recording(world: &mut IntegrationWorld) -> Result<()> {
    // The new recording is shown first; the sign covers it and gives it back after a second.
    let shown = ends_on_sign(run(world), "restart sign")?;
    if !shown.contains(&"recording") {
        bail!("expected the recording under the restart sign: {shown:?}");
    }
    Ok(())
}

#[then(expr = "the {word} sound plays and the overlay shows the {word} sign after the paste")]
pub(super) fn sound_and_sign_after_paste(
    world: &mut IntegrationWorld,
    sound: String,
    command: String,
) -> Result<()> {
    let run = run(world);
    let sign = sign_of(&command)?;
    let sound = sound_of(&sound)?;
    if !shown_after_paste(run, &[Event::Sound(sound), Event::Overlay(sign)])? {
        bail!(
            "expected the {sound} sound and {sign} after the paste: {:?}",
            run.events()
        );
    }
    Ok(())
}

/// Whether every one of `wanted` came after the paste.
fn shown_after_paste(run: &VoiceRun, wanted: &[Event]) -> Result<bool> {
    let pasted = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    for event in wanted {
        if run.wait_for(|e| e == event)? < pasted {
            return Ok(false);
        }
    }
    Ok(true)
}

#[then(expr = "the {word} sound plays and the overlay shows the {word} sign")]
pub(super) fn sound_and_sign(
    world: &mut IntegrationWorld,
    sound: String,
    command: String,
) -> Result<()> {
    let run = run(world);
    let (sound, sign) = (sound_of(&sound)?, sign_of(&command)?);
    run.wait_since_mark(|e| *e == Event::Sound(sound))?;
    run.wait_since_mark(|e| *e == Event::Overlay(sign))
}

#[then("the overlay shows the recording as soon as the new dictation starts")]
pub(super) fn recording_after_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Overlay("recording"))?;
    let shown = overlay_since_mark(run);
    let at = |wanted| shown.iter().position(|s| *s == wanted);
    match (at("enter sign"), at("recording")) {
        (Some(sign), Some(recording)) if sign < recording => Ok(()),
        _ => bail!("expected the enter sign, then the recording: {shown:?}"),
    }
}

#[then("the stop sound, the clear sound and the enter sound play in this order")]
pub(super) fn stop_clear_enter_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("enter"))?;
    let since = run.since_mark();
    let at = |name| since.iter().position(|e| *e == Event::Sound(name));
    match (at("stop"), at("clear"), at("enter")) {
        (Some(stop), Some(clear), Some(enter)) if stop < clear && clear < enter => Ok(()),
        _ => bail!("expected the stop, clear and enter sounds in order: {since:?}"),
    }
}

#[then("the overlay shows the clear sign, then the enter sign, then hides")]
pub(super) fn clear_then_enter_sign(world: &mut IntegrationWorld) -> Result<()> {
    let shown = ends_on_sign(run(world), "enter sign")?;
    match shown.iter().position(|s| *s == "clear sign") {
        Some(_) => Ok(()),
        None => bail!("expected the clear sign before the enter sign: {shown:?}"),
    }
}

#[when(expr = "the user says {string}, and later {string}")]
pub(super) fn says_and_later(
    world: &mut IntegrationWorld,
    first: String,
    second: String,
) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone(chain(&first).map_or(first.as_str(), |c| c[0].phrase()))?;
    std::thread::sleep(SETTLE);
    run.say_alone(chain(&second).map_or(second.as_str(), |c| c[0].phrase()))
}

#[then("the two commands play different sounds")]
pub(super) fn two_different_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let deadline = Instant::now() + REACTION;
    loop {
        let sounds: Vec<_> = run
            .since_mark()
            .into_iter()
            .filter(|e| matches!(e, Event::Sound(_)))
            .collect();
        if let [first, second] = sounds.as_slice() {
            if first == second {
                bail!("both commands played {first:?}");
            }
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("expected two command sounds: {sounds:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then("the start and stop sounds play")]
pub(super) fn start_and_stop_sounds(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("start"))?;
    run.wait_since_mark(|e| *e == Event::Sound("stop"))
}

#[then(
    "the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign"
)]
fn hotkey_overlay_without_sign(world: &mut IntegrationWorld) -> Result<()> {
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    let shown = ends_on_sign(run(world), PASTED_SIGN)?;
    if shown.iter().any(|s| COMMAND_SIGNS.contains(s)) {
        bail!("a command sign was shown: {shown:?}");
    }
    let at = |wanted| shown.iter().position(|s| *s == wanted);
    match (at("recording"), at("recognizing speech"), at(PASTED_SIGN)) {
        (Some(r), Some(p), Some(t)) if r < p && p < t => Ok(()),
        _ => bail!("expected recording, processing, the Pasted sign in order: {shown:?}"),
    }
}

/// The overlay event of a command's sign.
pub(super) fn sign_of(command: &str) -> Result<&'static str> {
    match command {
        "enter" => Ok("enter sign"),
        "clear" => Ok("clear sign"),
        "paste" => Ok("paste sign"),
        "restart" => Ok("restart sign"),
        other => bail!("no sign for {other}"),
    }
}

#[then("a new dictation starts after it")]
pub(super) fn new_dictation_after(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted_at = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let deadline = Instant::now() + REACTION;
    while !run.events()[pasted_at..].contains(&Event::RecordingStarted) {
        if Instant::now() >= deadline {
            bail!("no dictation started after the paste: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[then("the sentence is pasted, then select all and delete are pressed, then the Return key is pressed")]
pub(super) fn pasted_cleared_sent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let cleared = run.wait_for(|e| *e == Event::AllTextCleared)?;
    let sent = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if !(pasted < cleared && cleared < sent) {
        bail!("expected paste, clear, Return in order: {:?}", run.events());
    }
    Ok(())
}

#[then("no text is pasted")]
pub(super) fn no_text_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Pasted(_)), "text was pasted")
}
