//! The user's recorded chains whose last command counts only after the one before it ("start"
//! or "clear" while dictating), each played through the app: the chain's tail, decoded after
//! the commands before it were carried out, is still carried out.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{then, when};

use super::super::{carried_out_by_app, command, off_thread, silence};
use super::env;
use super::speech_after_start::{cut, forms_of, Form};
use crate::E2eWorld;

/// Silence before and after a chain, as between utterances.
const BEFORE_SECS: f32 = 1.0;
const AFTER_SECS: f32 = 1.5;
/// The user's labelled recording sessions holding chains.
const SESSIONS: [&str; 3] = ["session-2", "session-4", "session-5"];
/// The commands accepted while dictating only after the command before them.
const TAIL_ONLY_AFTER: [&str; 2] = ["start dictation", "clear all text"];

/// One recorded chain played through the app: its label and what the app carried out.
#[derive(Debug)]
pub struct RecordedChain {
    label: String,
    expected: Vec<VoiceCommand>,
    carried_out: Vec<VoiceCommand>,
}

#[when(
    "each of the user's recorded chains whose last command counts only after the one before it is played while dictating"
)]
async fn recorded_chains_played(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let url = world
            .voice
            .command_spotting_url
            .clone()
            .context("the command spotting service is not set up")?;
        let root = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
        let mut played = Vec::new();
        for session in SESSIONS {
            for form in forms_of(&root, session)?
                .iter()
                .filter(|form| ends_in_a_command_counting_only_after_another(form))
            {
                played.push(play(&url, &root, session, form)?);
            }
        }
        if played.is_empty() {
            bail!("no recorded chains ending in {TAIL_ONLY_AFTER:?} in {SESSIONS:?}");
        }
        world.voice.recorded_chains = played;
        Ok(())
    })
    .await
}

fn ends_in_a_command_counting_only_after_another(form: &Form) -> bool {
    form.state == "dictating"
        && form.commands.len() >= 2
        && form
            .commands
            .last()
            .is_some_and(|last| TAIL_ONLY_AFTER.contains(&last.as_str()))
}

/// `form` cut from its session with silence around it, through the app while dictating.
fn play(url: &str, root: &Path, session: &str, form: &Form) -> Result<RecordedChain> {
    let mut audio = silence(BEFORE_SECS);
    audio.extend(cut(root, session, form)?);
    audio.extend(silence(AFTER_SECS));
    let expected = form
        .commands
        .iter()
        .map(|phrase| command(phrase))
        .collect::<Result<Vec<_>>>()?;
    let carried_out = carried_out_by_app(url, &audio, &VoiceCommand::DICTATING)?;
    Ok(RecordedChain {
        label: format!("{session} \"{}\" at {} s", form.said, form.start),
        expected,
        carried_out,
    })
}

#[then("every command of each chain takes effect in order, each once")]
fn every_command_of_each_chain(world: &mut E2eWorld) -> Result<()> {
    let wrong: Vec<String> = world
        .voice
        .recorded_chains
        .iter()
        .filter(|chain| chain.carried_out != chain.expected)
        .map(|chain| {
            format!(
                "{}: carried out {:?}, expected {:?}",
                chain.label, chain.carried_out, chain.expected
            )
        })
        .collect();
    if !wrong.is_empty() {
        bail!("{}", wrong.join("\n"));
    }
    Ok(())
}
