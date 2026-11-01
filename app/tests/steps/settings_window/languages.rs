//! The spoken language choice: what it offers and what it keeps.

use super::{ensure, given, settings, then, Choices, IntegrationWorld, Result};

/// A configuration file that sets other options but no spoken language.
const WITHOUT_LANGUAGE: &str = "[logging]\nlevel = \"info\"\n";
const GERMAN: &str = "de";

#[given("the configuration file names no spoken language")]
pub(super) fn file_without_language(world: &mut IntegrationWorld) {
    settings(world).write(WITHOUT_LANGUAGE);
}

#[given("the configuration file names German as the spoken language")]
pub(super) fn file_with_german(world: &mut IntegrationWorld) {
    settings(world).write(&format!("[transcription]\nlanguage = \"{GERMAN}\"\n"));
}

#[then(
    "the spoken language choice offers Russian, English and automatic detection, and nothing else"
)]
pub(super) fn offers_three(world: &mut IntegrationWorld) -> Result<()> {
    let offered = settings(world).session()?.options(Choices::Languages);
    let entries: Vec<(&str, Option<&str>)> = offered
        .iter()
        .map(|c| (c.title.as_str(), c.value.as_str()))
        .collect();
    ensure!(
        entries
            == [
                ("Russian", Some("ru")),
                ("English", Some("en")),
                ("Detect automatically", Some("auto"))
            ],
        "{entries:?}"
    );
    Ok(())
}

#[then("the spoken language control shows Russian")]
pub(super) fn shows_russian(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("transcription", "language")?;
    ensure!(shown == "Russian", "shows {shown}");
    Ok(())
}

#[then("the spoken language control shows the German code")]
pub(super) fn shows_german_code(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("transcription", "language")?;
    ensure!(shown == GERMAN, "shows {shown}");
    Ok(())
}

#[then("the configuration file still names German")]
pub(super) fn file_names_german(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(
        saved.transcription.language == GERMAN,
        "{}",
        saved.transcription.language
    );
    Ok(())
}
