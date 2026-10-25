//! How soon each of the user's recorded starts from standby gives its start cue, on its own
//! and followed at once by a sentence.

use voicepuncher::test_doubles::prelude::*;

use cucumber::when;

use super::speech_after_start::{cut, forms_of, Form};
use super::start_cue::sentence_at_once;
use super::{env, room_noise, session_of, until_last_sound, HandsFreeSession};
use crate::E2eWorld;

/// Room noise before each of the user's recorded starts, as in a real stream.
const ROOM_BEFORE_SECS: f32 = 1.0;
/// The user's labelled recording session holding their starts from standby.
const RECORDED_STARTS_SESSION: &str = "session-2";
/// The ways of saying a start from standby that are only the command's words.
const START_FORMS: [&str; 3] = ["start", "start start", "start please"];

#[when(
    "each of the user's recorded starts is played, on its own and followed at once by a sentence"
)]
fn recorded_starts(world: &mut E2eWorld) -> Result<()> {
    let session = session_of(world)?;
    let root = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    let forms = recorded_start_forms(&root)?;
    let room_sound = room_noise(0.8)?;
    for form in &forms {
        for followed in [false, true] {
            time_recorded_start(session, &root, form, followed, &room_sound)?;
        }
    }
    Ok(())
}

/// The user's recorded starts said on their own in standby.
fn recorded_start_forms(root: &Path) -> Result<Vec<Form>> {
    let forms: Vec<_> = forms_of(root, RECORDED_STARTS_SESSION)?
        .into_iter()
        .filter(|form| {
            form.state == "standby"
                && form.commands == ["start dictation"]
                && START_FORMS.contains(&form.said.as_str())
        })
        .collect();
    if forms.is_empty() {
        bail!("no recorded starts in {RECORDED_STARTS_SESSION}");
    }
    Ok(forms)
}

/// Times the start of `form`, alone or `followed` at once by a sentence; a dictation it
/// started is cancelled.
fn time_recorded_start(
    session: &HandsFreeSession,
    root: &Path,
    form: &Form,
    followed: bool,
    room_sound: &[f32],
) -> Result<()> {
    let mut audio = room_noise(ROOM_BEFORE_SECS)?;
    audio.extend(until_last_sound(cut(root, RECORDED_STARTS_SESSION, form)?));
    let words_end = audio.len();
    let after = if followed {
        audio.extend(sentence_at_once()?);
        "followed by speech"
    } else {
        "alone"
    };
    let label = format!("recorded \"{}\" at {} s {after}", form.said, form.start);
    let start = session.time_start(label, &audio, words_end, room_sound)?;
    if start.cue.is_some() {
        session.cancel_dictation(room_sound)?;
    }
    Ok(())
}
