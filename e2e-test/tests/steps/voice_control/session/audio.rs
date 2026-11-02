//! The audio of a hands-free session: recordings, room noise and command audio.

use super::{
    env, read_wav, recorded_form, spoken_with_pauses, CommandVoice, Expected, PathBuf, Result,
    Step, CANCELLED_EN, CLEARED_EN, COMMAND_PAUSE_SECS, ENGLISH_VOICE, RUSSIAN_VOICE,
    SAMPLE_RATE_HZ, SENT_AGAIN_RU, SENT_RU,
};
use voicepuncher_e2e_test::conversions::saturating_usize;

/// Send, cancel, clear, send again.
pub(super) fn session() -> Vec<Step> {
    use Expected::{Cancelled, Cleared, Pasted, RecordingStarted, ReturnPressed};
    vec![
        Step::Command("start", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_RU),
        Step::Command("stop", Pasted(SENT_RU)),
        Step::Command("enter", ReturnPressed),
        Step::Command("start", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CANCELLED_EN),
        Step::Command("cancel", Cancelled),
        Step::Command("start", RecordingStarted),
        Step::Dictate(ENGLISH_VOICE, CLEARED_EN),
        Step::Command("stop", Pasted(CLEARED_EN)),
        Step::Command("clear", Cleared),
        Step::Command("start", RecordingStarted),
        Step::Dictate(RUSSIAN_VOICE, SENT_AGAIN_RU),
        Step::Command("stop", Pasted(SENT_AGAIN_RU)),
        Step::Command("enter", ReturnPressed),
    ]
}

pub(super) fn room_noise(secs: f32) -> Result<Vec<f32>> {
    let path = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?).join("session-1.wav");
    let room = to_f32(read_wav(&path)?);
    let quiet =
        &room[saturating_usize(0.5 * SAMPLE_RATE_HZ)..saturating_usize(4.5 * SAMPLE_RATE_HZ)];
    Ok(quiet
        .iter()
        .copied()
        .cycle()
        .take(saturating_usize(secs * SAMPLE_RATE_HZ))
        .collect())
}

pub(super) fn to_f32(samples: Vec<i16>) -> Vec<f32> {
    samples
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect()
}

pub(super) fn command_audio(voice: CommandVoice, said: &str) -> Result<Vec<f32>> {
    match voice {
        // The default voice: Eddy's "cancel" is heard as "capsule", alone or doubled.
        CommandVoice::Synthesized => spoken_with_pauses(said),
        CommandVoice::Recorded { .. } => {
            let mut audio = room_noise(COMMAND_PAUSE_SECS)?;
            audio.extend(recorded_or_twice(said)?);
            audio.extend(room_noise(COMMAND_PAUSE_SECS)?);
            Ok(audio)
        }
    }
}

/// The user's recorded `said`; a doubled word the user never recorded doubled is their
/// recording of the word, said twice.
fn recorded_or_twice(said: &str) -> Result<Vec<f32>> {
    match said.split_once(' ') {
        Some((first, second)) if first == second && recorded_form(said).is_err() => {
            let mut audio = recorded_form(first)?;
            audio.extend(recorded_form(first)?);
            Ok(audio)
        }
        _ => recorded_form(said),
    }
}

/// The words a session says for the command `said`. A lone "start" right after another
/// command is often too short for the voice gate, the user's recorded one too: the keyword
/// spotter hears it, no speech segment closes, so nothing confirms it. So the session says
/// "start" doubled in either voice, and a synthesized voice doubles "stop" and "cancel" as
/// well. The user's recorded "enter" (0.27 s) after a long pause is sometimes not heard at
/// all, no trigger and no segment, so a recorded session doubles it too; "clear" alone is
/// heard.
pub(super) fn session_form(voice: CommandVoice, said: &str) -> String {
    let doubled = match voice {
        CommandVoice::Synthesized => ["start", "stop", "cancel"].contains(&said),
        CommandVoice::Recorded { .. } => ["start", "enter"].contains(&said),
    };
    if doubled {
        format!("{said} {said}")
    } else {
        said.to_owned()
    }
}
