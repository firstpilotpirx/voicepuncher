//! Then steps: what was pasted, pressed and shown.

use super::{
    bail, given, is_key_press, pipeline_log, run, then, Duration, Event, Instant, IntegrationWorld,
    Result, MORE_WORDS, PASTED_SIGN, REACTION, SENTENCE, SETTLE, STOP_PHRASE,
};

#[then("nothing was pasted before the dictation was ended")]
pub(super) fn nothing_pasted_before_end(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let before = &run.events()[..run.mark];
    if let Some(paste) = before.iter().find(|e| matches!(e, Event::Pasted(_))) {
        bail!("pasted before the dictation was ended: {paste:?}; events: {before:?}");
    }
    run.wait_since_mark(|e| matches!(e, Event::Pasted(_)))
}

pub(crate) fn no_listening_sound_after_start(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let events = run.events();
    let listening_sounds = events
        .iter()
        .filter(|e| matches!(e, Event::Sound("listening on" | "listening off")))
        .count();
    if listening_sounds != 1 {
        bail!("expected only the listening-on sound at start; events: {events:?}");
    }
    Ok(())
}

#[then("a dictation starts")]
pub(super) fn dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[then("listening is on")]
#[then("listening stays on")]
pub(super) fn listening_on(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    std::thread::sleep(SETTLE);
    if !run.listening() {
        bail!("listening is off; events: {:?}", run.events());
    }
    Ok(())
}

#[then(expr = "saying {string} presses Return")]
pub(super) fn saying_presses_return(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    run(world).start_when();
    run(world).say_alone(&phrase)?;
    run(world).wait_since_mark(|e| *e == Event::ReturnPressed)
}

#[then("the overlay is hidden")]
pub(super) fn overlay_hidden(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::Overlay("hidden"))
}

#[then("the overlay shows the recording")]
pub(super) fn overlay_recording(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::Overlay("recording"))
}

#[then("the overlay shows processing until the sentence is pasted, then hides")]
pub(super) fn overlay_processing_then_hidden(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    run.wait_since_mark(|e| *e == Event::Overlay(PASTED_SIGN))?;
    let since = run.since_mark();
    let at = |wanted: &Event| since.iter().position(|e| e == wanted);
    // processing starts with its first stage, speech recognition
    let processing = at(&Event::Overlay("recognizing speech"));
    let pasted = since.iter().position(|e| matches!(e, Event::Pasted(_)));
    let signed = at(&Event::Overlay(PASTED_SIGN));
    match (processing, pasted, signed) {
        (Some(p), Some(t), Some(s)) if p < t && t < s => Ok(()),
        _ => bail!("expected processing, paste, the Pasted sign in order; events: {since:?}"),
    }
}

#[then(expr = "the {word} sound plays")]
pub(super) fn sound_plays(world: &mut IntegrationWorld, sound: String) -> Result<()> {
    let name = sound_of(&sound)?;
    run(world).wait_since_mark(|e| *e == Event::Sound(name))
}

/// Each sound as a scenario names it, with its event name.
const SOUNDS: [(&str, &str); 9] = [
    ("listening-on", "listening on"),
    ("listening-off", "listening off"),
    ("start", "start"),
    ("stop", "stop"),
    ("cancel", "cancel"),
    ("enter", "enter"),
    ("clear", "clear"),
    ("paste", "paste"),
    ("restart", "restart"),
];

/// The event name of a sound as a scenario names it.
pub(super) fn sound_of(sound: &str) -> Result<&'static str> {
    SOUNDS
        .iter()
        .find(|(named, _)| *named == sound)
        .map(|(_, event)| *event)
        .ok_or_else(|| anyhow::anyhow!("unknown sound {sound}"))
}

#[then("no sound plays")]
pub(super) fn no_sound(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Sound(_)), "a sound played")
}

pub(crate) fn sentence_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if !pasted.contains(SENTENCE) {
        bail!("pasted {pasted:?}, expected the sentence {SENTENCE:?}");
    }
    Ok(())
}

#[then("the pasted text does not contain the stop phrase")]
pub(super) fn no_stop_phrase(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if pasted.contains(STOP_PHRASE) {
        bail!("the stop phrase reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then(expr = "the pasted text does not contain {string}")]
pub(super) fn pasted_without(world: &mut IntegrationWorld, unwanted: String) -> Result<()> {
    let pasted = run(world).pasted()?;
    let has_word = pasted
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .any(|w| w == unwanted);
    if has_word {
        bail!("{unwanted:?} reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then("the pasted text does not contain the words after it")]
pub(super) fn pasted_without_more_words(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    if pasted.contains(MORE_WORDS) {
        bail!("the words after the command reached the pasted text: {pasted:?}");
    }
    Ok(())
}

#[then("a new dictation starts")]
pub(super) fn new_dictation_starts(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::RecordingStarted)
}

pub(crate) fn return_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ReturnPressed)?;
    Ok(())
}

#[given("the clipboard holds a sentence")]
pub(super) fn clipboard_holds_sentence(_world: &mut IntegrationWorld) {
    // The app never reads the clipboard: what it holds is the focused app's to paste.
}

#[then("Cmd+V is pressed in the focused app")]
pub(super) fn paste_key_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::PasteKeyPressed)?;
    Ok(())
}

#[then("the clipboard still holds the same sentence")]
pub(super) fn clipboard_untouched(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|e| *e == Event::PasteKeyPressed)?;
    run.never_since_mark(
        |e| matches!(e, Event::Pasted(_)),
        "the clipboard was written",
    )
}

#[then("Cmd+V is pressed, then the Return key is pressed")]
pub(super) fn pasted_clipboard_then_sent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted = run.wait_for(|e| *e == Event::PasteKeyPressed)?;
    let sent = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if pasted >= sent {
        bail!("expected Cmd+V, then Return: {:?}", run.events());
    }
    Ok(())
}

#[then("select all and delete are pressed in the focused app")]
pub(super) fn all_text_cleared(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::AllTextCleared)?;
    Ok(())
}

#[then("the dictation keeps running")]
pub(super) fn dictation_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let ended = |e: &Event| {
        matches!(
            e,
            Event::Pasted(_) | Event::RecordingCancelled | Event::Sound("stop")
        )
    };
    run(world).never_since_mark(ended, "the dictation ended")
}

#[then("no key is pressed")]
pub(super) fn no_key_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(is_key_press, "a key was pressed")
}

#[then("the text is pasted first")]
pub(super) fn text_pasted_first(world: &mut IntegrationWorld) -> Result<()> {
    run(world).pasted()?;
    Ok(())
}

pub(crate) fn return_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let pasted_at = run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    let return_at = run.wait_for(|e| *e == Event::ReturnPressed)?;
    if return_at < pasted_at {
        bail!("Return was pressed before the paste: {:?}", run.events());
    }
    Ok(())
}

#[then("an error naming the command spotting service is logged")]
pub(super) fn error_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let deadline = Instant::now() + REACTION;
    loop {
        let errors = pipeline_log().errors.lock().unwrap().clone();
        if errors
            .iter()
            .any(|e| e.contains("command spotting service"))
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("no error naming the command spotting service; errors: {errors:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
