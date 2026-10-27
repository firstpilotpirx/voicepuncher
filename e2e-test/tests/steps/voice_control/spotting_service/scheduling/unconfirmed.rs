//! Commands reported without the confirmation of the speech recognition service for
//! commands: what goes out at once, and the verdict of its hearing that follows.

use cucumber::{given, then};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_command_spotting::protocol::{ServerEvent, Verdict};

use super::run;
use crate::E2eWorld;

#[given("the confirmation of commands is off")]
fn confirmation_off(world: &mut E2eWorld) {
    run(world).unconfirmed = true;
    world.voice.unconfirmed_commands = true;
}

/// The verdict a scenario names: "agreed", "other" or "none".
fn verdict_named(name: &str) -> Result<Verdict> {
    Ok(match name {
        "agreed" => Verdict::Agreed,
        "other" => Verdict::Other,
        "none" => Verdict::None,
        _ => bail!("no verdict {name:?}"),
    })
}

/// Whether `event` reports `command` before any hearing confirmed it.
fn reported_unconfirmed(event: &ServerEvent, command: &str) -> bool {
    match event {
        ServerEvent::Phrase { phrase, timing: None, .. } => phrase == command,
        ServerEvent::Command { commands, timing: None, .. } => {
            commands.iter().any(|c| c == command)
        }
        _ => false,
    }
}

/// Whether `event` reports `command`, confirmed or not.
fn reported_as(event: &ServerEvent, command: &str) -> bool {
    match event {
        ServerEvent::Phrase { phrase, .. } => phrase == command,
        ServerEvent::Command { commands, .. } => commands.iter().any(|c| c == command),
        _ => false,
    }
}

/// Fails unless `events` report `command` exactly once, confirmed or not.
fn reported_once(events: &[ServerEvent], command: &str) -> Result<()> {
    let reports = events.iter().filter(|e| reported_as(e, command)).count();
    if reports != 1 {
        bail!("expected {command:?} reported once, got {reports} reports: {events:?}");
    }
    Ok(())
}

#[then(
    expr = "{string} is reported once, before its hearing, and Whisper's verdict on it, {string}, follows in the log and the command history"
)]
fn reported_then_verdict(world: &mut E2eWorld, command: String, verdict: String) -> Result<()> {
    let expected = verdict_named(&verdict)?;
    let run = run(world);
    let events = run.runs.concat();
    let reported = events
        .iter()
        .position(|e| reported_unconfirmed(e, &command));
    reported_once(&events, &command)?;
    let heard = events.iter().position(|e| {
        matches!(e, ServerEvent::Heard { commands, verdict, .. }
            if commands.contains(&command) && *verdict == expected)
    });
    if reported
        .zip(heard)
        .is_none_or(|(said, heard)| said >= heard)
    {
        bail!("expected {command:?} reported at once, then the verdict {verdict:?}: {events:?}");
    }
    verdict_recorded(&run.history.only_entry()?, &verdict)
}

/// Fails unless the command history `entry` holds `verdict` for one of its reports.
fn verdict_recorded(entry: &serde_json::Value, verdict: &str) -> Result<()> {
    let recorded = entry["reported"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|r| r["whisper"] == verdict);
    if !recorded {
        bail!("the command history entry does not hold the verdict {verdict:?}: {entry}");
    }
    Ok(())
}

#[then(expr = "{string} is reported only once its hearing confirmed it")]
fn reported_confirmed(world: &mut E2eWorld, command: String) -> Result<()> {
    let events = run(world).runs.concat();
    let confirmed = events.iter().any(|e| match e {
        ServerEvent::Phrase { phrase, timing: Some(_), .. } => *phrase == command,
        ServerEvent::Command { commands, timing: Some(_), .. } => commands.contains(&command),
        _ => false,
    });
    // A command reported without confirmation always gets Whisper's verdict afterwards; a
    // spotted phrase held with its segment until the confirmation is no such report.
    let verdict = events.iter().any(|e| match e {
        ServerEvent::Heard { commands, .. } => commands.contains(&command),
        _ => false,
    });
    if !confirmed || verdict {
        bail!("expected {command:?} reported only after its hearing: {events:?}");
    }
    Ok(())
}
