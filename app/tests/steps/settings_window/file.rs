//! Then steps: what the configuration file holds and the window's tabs and notes.

use super::{
    bail, ensure, files, settings, then, Field, IntegrationWorld, Result, Tab, COMMAND_LINE_HOTKEY,
    FIELDS, SECOND_HOTKEY,
};

#[then("the configuration file is unchanged")]
pub(super) fn file_unchanged(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    ensure!(Some(run.file()?) == run.original, "the file changed");
    ensure!(
        run.statuses.lock().unwrap().is_empty(),
        "something was applied"
    );
    Ok(())
}

#[then("the file differs from the original only in the logging level")]
pub(super) fn differs_only_in_level(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let expected = files::COMMENTED.replace("level = \"info\"", "level = \"debug\"");
    ensure!(run.file()? == expected, "file:\n{}", run.file()?);
    Ok(())
}

#[then("every comment is where it was")]
pub(super) fn comments_kept(world: &mut IntegrationWorld) -> Result<()> {
    let file = settings(world).file()?;
    let comments = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.find('#').map(|at| line[at..].to_owned()))
            .collect()
    };
    ensure!(
        comments(&file) == comments(files::COMMENTED),
        "file:\n{file}"
    );
    Ok(())
}

#[then("the retired text refinement settings in the file are unchanged")]
pub(super) fn retired_settings_kept(world: &mut IntegrationWorld) -> Result<()> {
    let file = settings(world).file()?;
    ensure!(file.contains(files::RETIRED_BLOCK), "file:\n{file}");
    Ok(())
}

#[then("the tabs are General, Dictation, Sounds, Paste, Archive, Logging and Services")]
pub(super) fn tabs_listed(_world: &mut IntegrationWorld) -> Result<()> {
    let titles: Vec<&str> = Tab::ALL.iter().map(|tab| tab.title()).collect();
    ensure!(
        titles
            == [
                "General",
                "Dictation",
                "Sounds",
                "Paste",
                "Archive",
                "Logging",
                "Services"
            ],
        "tabs {titles:?}"
    );
    Ok(())
}

#[then("no control names text refinement")]
pub(super) fn no_refinement_control(_world: &mut IntegrationWorld) -> Result<()> {
    let named: Vec<String> = FIELDS
        .iter()
        .filter(|field| {
            let words = format!("{} {}", field.path(), field.label).to_lowercase();
            words.contains("refine") || words.contains("llm")
        })
        .map(Field::path)
        .collect();
    ensure!(named.is_empty(), "controls name text refinement: {named:?}");
    Ok(())
}

#[then("the settings ask whether to save, not save or cancel")]
pub(super) fn asks_on_close(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        settings(world).session()?.is_dirty(),
        "closing would ask nothing"
    );
    Ok(())
}

#[then("choosing cancel keeps the settings open with the edit")]
pub(super) fn cancel_keeps_edit(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "key")?;
    ensure!(shown == SECOND_HOTKEY.human_name(), "shows {shown}");
    Ok(())
}

#[then("the settings close without a question")]
pub(super) fn closes_quietly(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(!settings(world).session()?.is_dirty(), "closing would ask");
    Ok(())
}

#[then("a note under it names F13 as the hotkey the command line set for this run")]
pub(super) fn hotkey_note(world: &mut IntegrationWorld) -> Result<()> {
    note_for(world, "hotkey.key", COMMAND_LINE_HOTKEY.human_name())
}

pub(super) fn note_for(world: &mut IntegrationWorld, path: &str, value: &str) -> Result<()> {
    let notes = settings(world).session()?.notes();
    let Some(note) = notes.iter().find(|n| n.field.path() == path) else {
        bail!("no note for {path}: {notes:?}");
    };
    ensure!(
        note.note.contains(value) && note.note.contains("command line"),
        "{}",
        note.note
    );
    Ok(())
}
