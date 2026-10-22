//! The configuration files the settings scenarios start from.

use std::collections::BTreeSet;

use anyhow::{ensure, Result};

use voicepuncher::adapters::primary::settings::macos::table::{FIELDS, KEPT, RETIRED};
use voicepuncher::config::Config;

pub const RIGHT_COMMAND: &str = "[hotkey]\nkey = \"RightCommand\"\n";

pub const WITHOUT_MIN_PRESS: &str = "[hotkey]\nkey = \"RightOption\"\n";

pub const VALID_MIN_PRESS: &str = "[hotkey]\nmin_press_ms = 300\n";

pub const REFINEMENT_ON: &str = "[refinement]\nenabled = true\n";

/// The app's settings in the app-runtime scenarios: the first hotkey, a press of any length,
/// a press starts and stops, the first start sound, no voice control, no refinement.
pub const FIRST_SETTINGS: &str = "\
[hotkey]
key = \"RightOption\"
min_press_ms = 0
recording_mode = \"Toggle\"

[sounds]
start_sound = \"Glass\"

[voice_control]
enabled = false

[refinement]
enabled = false
";

pub const COMMENTED: &str = "\
# voicepuncher settings, kept by hand

[hotkey]
# the key that starts a dictation
key = \"RightOption\"  # right Option
min_press_ms = 300

[logging]
# how much the log says
level = \"info\"  # info is enough day to day
";

pub const EXAMPLES_BLOCK: &str = "\
[[refinement.examples]]
transcript = \"meeting moved to friday\"
normalized = \"Meeting moved to Friday.\"
";

pub const WITH_EXAMPLES: &str = concat!(
    "[logging]\nlevel = \"info\"\n\n",
    "[[refinement.examples]]\n",
    "transcript = \"meeting moved to friday\"\n",
    "normalized = \"Meeting moved to Friday.\"\n",
);

/// Every key the configuration accepts, except the retired ones and the app's own records,
/// has exactly one field.
pub fn every_key_has_one_field() -> Result<()> {
    let toml::Value::Table(sections) = toml::Value::try_from(Config::default())? else {
        anyhow::bail!("the configuration is not a table");
    };
    let retired: BTreeSet<String> = RETIRED
        .iter()
        .chain(KEPT)
        .map(|(s, k)| format!("{s}.{k}"))
        .collect();
    let keys: BTreeSet<String> = sections
        .iter()
        .flat_map(|(section, keys)| {
            let keys = keys.as_table().cloned().unwrap_or_default();
            keys.into_iter()
                .map(move |(key, _)| format!("{section}.{key}"))
        })
        .filter(|path| !retired.contains(path))
        .collect();
    let paths: Vec<String> = FIELDS
        .iter()
        .map(voicepuncher::adapters::primary::settings::macos::field::Field::path)
        .collect();
    let fields: BTreeSet<String> = paths.iter().cloned().collect();
    ensure!(
        fields.len() == paths.len(),
        "a key has two fields: {paths:?}"
    );
    ensure!(fields == keys, "fields {fields:?}, keys {keys:?}");
    Ok(())
}

/// No field edits the app's own records, such as whether the login question was asked.
pub fn no_field_edits_the_app_s_records() -> Result<()> {
    let edited: Vec<String> = FIELDS
        .iter()
        .map(voicepuncher::adapters::primary::settings::macos::field::Field::path)
        .filter(|path| KEPT.iter().any(|(s, k)| *path == format!("{s}.{k}")))
        .collect();
    ensure!(
        edited.is_empty(),
        "fields edit the app's records: {edited:?}"
    );
    ensure!(KEPT.contains(&("app", "launch_at_login_asked")), "{KEPT:?}");
    Ok(())
}
