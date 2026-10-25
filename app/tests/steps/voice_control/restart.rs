//! Steps of restarting a dictation by voice.

use super::{
    bail, dictation_runs, given, run, short_form, then, Context, Duration, Event, Instant,
    IntegrationWorld, Result, VoiceRun, REACTION, SENTENCE,
};

/// Said after a restart: the only text of the dictation that follows.
pub(super) const OTHER_SENTENCE: &str = "Lunch is at noon.";

/// Words of the commands that start a dictation over: none may be pasted.
pub(super) const RESTART_WORDS: [&str; 3] = ["restart", "start", "dictation"];

#[given(expr = "the user said a sentence, {string}, and another sentence")]
pub(super) fn restarted_with_another_sentence(
    world: &mut IntegrationWorld,
    restart: String,
) -> Result<()> {
    dictation_runs(world)?;
    let run = run(world);
    run.speak(SENTENCE, Instant::now());
    run.say_alone(short_form(&restart).context("no command")?.phrase())?;
    let deadline = Instant::now() + REACTION;
    let started = |run: &VoiceRun| {
        run.events()
            .iter()
            .filter(|e| **e == Event::RecordingStarted)
            .count()
    };
    while started(run) < 2 {
        if Instant::now() >= deadline {
            bail!("no new dictation after the restart: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    run.speak(OTHER_SENTENCE, Instant::now());
    Ok(())
}

#[then("only the other sentence is pasted")]
pub(crate) fn only_other_sentence(world: &mut IntegrationWorld) -> Result<()> {
    let pasted = run(world).pasted()?;
    let command_word = pasted
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .any(|w| RESTART_WORDS.contains(&w));
    if command_word || pasted.contains(SENTENCE) || !pasted.contains(OTHER_SENTENCE) {
        bail!("pasted {pasted:?}, expected only {OTHER_SENTENCE:?}");
    }
    Ok(())
}

#[then("the restart sound plays, and neither the cancel sound nor the start sound")]
pub(super) fn restart_sound_alone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::Sound("restart"))?;
    run.never_since_mark(
        |e| matches!(e, Event::Sound("cancel" | "start")),
        "a cancel or start sound played",
    )
}
