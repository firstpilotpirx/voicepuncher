//! Then steps: what the archive's entries hold.

use super::{
    bail, entry_folders, run, then, Event, IntegrationWorld, Result, CHUNK_TEXTS, DICTATION_MODEL,
    NOT_ARCHIVED_WARNING, PHRASE, SECOND_PHRASE, SETTLE, STITCHED_CHUNKS,
};

#[then("the archive holds one entry, with the audio of the restarted dictation only")]
pub(super) fn restarted_dictation_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    std::thread::sleep(SETTLE);
    let folders = entry_folders(&run.archive_dir()?)?;
    if folders.len() != 1 {
        bail!("expected only the restarted dictation's entry, found {folders:?}");
    }
    entry.expect_audio(run.dictated(1)?)?;
    entry.expect("start", "voice")
}

#[then(
    "a new archive entry holds the audio of the dictation, the raw transcript speech recognition returned, and the text that was pasted"
)]
fn entry_with_audio_and_texts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("raw_transcript", PHRASE)?;
    entry.expect("pasted_text", PHRASE)?;
    entry.expect("outcome", "pasted")?;
    entry.expect("start", "hotkey_toggled")
}

#[then(
    "its archive entry holds the text of each chunk in the order spoken, the stitched raw transcript and the audio of the whole dictation"
)]
fn entry_with_every_chunk(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect("chunks", CHUNK_TEXTS.to_vec())?;
    entry.expect("raw_transcript", STITCHED_CHUNKS)?;
    entry.expect_audio(run.dictated(0)?)
}

#[then(
    "the archive holds two entries, each with only its own audio and texts, the later one sorting after the earlier one"
)]
fn two_entries(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entries = run.entries(2)?;
    for (number, (entry, phrase)) in entries.iter().zip([PHRASE, SECOND_PHRASE]).enumerate() {
        entry.expect_audio(run.dictated(number)?)?;
        entry.expect("chunks", vec![phrase])?;
        entry.expect("raw_transcript", phrase)?;
        entry.expect("pasted_text", phrase)?;
    }
    Ok(())
}

#[then("its entry names no text refinement model and no refinement outcome")]
pub(super) fn entry_without_refinement(world: &mut IntegrationWorld) -> Result<()> {
    let entry = run(world).only_entry()?;
    for key in ["refinement", "refinement_model"] {
        if entry.json.get(key).is_some() {
            bail!("the entry still has {key:?}: {}", entry.json);
        }
    }
    entry.expect_pasted_raw_transcript()
}

#[then("its entry holds the audio and says speech recognition failed, with no pasted text")]
pub(super) fn recognition_failed_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("outcome", "speech_recognition_failed")?;
    entry.expect("pasted_text", serde_json::Value::Null)
}

#[then("its entry holds the audio and says only command words were transcribed")]
pub(super) fn only_command_words_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let entry = run.only_entry()?;
    entry.expect_audio(run.dictated(0)?)?;
    entry.expect("outcome", "only_command_words")?;
    entry.expect("pasted_text", serde_json::Value::Null)
}

#[then("the archive holds no entry for it")]
pub(super) fn no_entry(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until("the overlay hiding", |run| {
        Ok(run.events().contains(&Event::Hidden))
    })?;
    std::thread::sleep(SETTLE);
    let folders = entry_folders(&run.archive_dir()?)?;
    if !folders.is_empty() {
        bail!("a dropped dictation left entries: {folders:?}");
    }
    Ok(())
}

#[then("its entry says it started by voice and names the app version and the dictation model")]
pub(super) fn voice_started_entry(world: &mut IntegrationWorld) -> Result<()> {
    let entry = run(world).only_entry()?;
    entry.expect("start", "voice")?;
    entry.expect("app_version", env!("CARGO_PKG_VERSION"))?;
    entry.expect("dictation_model", DICTATION_MODEL)?;
    entry.expect("outcome", "pasted")
}

#[then("the phrase is pasted as usual and the log warns that the dictation was not archived")]
pub(super) fn pasted_and_warned(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until("the warning", |run| {
        Ok(run
            .warnings()
            .iter()
            .any(|line| line.starts_with(NOT_ARCHIVED_WARNING)))
    })?;
    if run.pasted() != [PHRASE] {
        bail!("the phrase was not pasted as usual: {:?}", run.events());
    }
    if !run.is_unwritable() {
        bail!("the archive folder was writable");
    }
    Ok(())
}
