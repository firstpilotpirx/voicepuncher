//! One opening of the settings window without its controls: the values shown, the user's
//! edits, and what Save, Revert and closing do with them.

use std::path::{Path, PathBuf};

use anyhow::Result;
use toml::Value;

use super::field::{Choices, Field};
use super::overrides::{override_notes, OverrideNote};
use super::validation::validate;
use super::values::{options, shown_value, to_item, Choice};
use crate::config::document::ConfigDocument;
use crate::config::Config;
use crate::core::application::InputDevices;

/// What became of saved settings in the running app, as it happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyStatus {
    /// A dictation is in progress; the settings apply once it has ended.
    WaitsForDictation,
    /// The app runs with the saved settings.
    Applied,
    /// The saved settings were not applied, for this reason; the app keeps its settings.
    Rejected(String),
}

/// Told every [`ApplyStatus`] of one Save, from any thread.
pub type StatusListener = Box<dyn FnMut(ApplyStatus) + Send>;

/// Applies a saved configuration to the running app.
pub trait ConfigApplier: Send {
    /// Applies `config` once the dictation in progress has ended, telling `listener`.
    fn apply(&self, config: Config, listener: StatusListener);
}

/// This run's command line options, applied over a configuration read from the file.
pub type CommandLineOverlay = Box<dyn Fn(&mut Config) + Send>;

/// Why Save wrote nothing: the field to mark, when one is at fault, and the message.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub field: Option<&'static Field>,
    pub message: String,
}

/// What the session needs from the app around it.
pub struct SessionParts {
    pub overlay: CommandLineOverlay,
    pub applier: Box<dyn ConfigApplier>,
    pub devices: Box<dyn InputDevices>,
}

/// The settings of one configuration file as the window shows and edits them.
pub struct SettingsSession {
    path: PathBuf,
    home: PathBuf,
    document: ConfigDocument,
    saved: Config,
    edits: Vec<(&'static Field, Value)>,
    parts: SessionParts,
}

impl SettingsSession {
    /// The settings of the file at `path` (none yet when it does not exist); `home`
    /// resolves folders written with `~`.
    pub fn open(path: &Path, home: &Path, parts: SessionParts) -> Result<Self> {
        let document = ConfigDocument::load(path)?;
        let saved = document.to_config()?;
        Ok(Self {
            path: path.to_path_buf(),
            home: home.to_path_buf(),
            document,
            saved,
            edits: Vec::new(),
            parts,
        })
    }

    /// The value `field` shows: the user's edit, else the file's value or its default.
    pub fn value(&self, field: &Field) -> Value {
        self.edits
            .iter()
            .find(|(edited, _)| edited.path() == field.path())
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| shown_value(&self.saved, field))
    }

    /// The entries a pop-up or a pair of radio buttons offers.
    pub fn options(&self, choices: Choices) -> Vec<Choice> {
        let microphones = match choices {
            Choices::Microphones => self.parts.devices.names(),
            _ => Vec::new(),
        };
        options(choices, &microphones)
    }

    /// The user set `field` to `value` in its control; setting the file's value again
    /// drops the edit.
    pub fn edit(&mut self, field: &'static Field, value: Value) {
        self.edits
            .retain(|(edited, _)| edited.path() != field.path());
        if value != shown_value(&self.saved, field) {
            self.edits.push((field, value));
        }
    }

    /// Whether a value differs from the file's; closing the window then asks first.
    pub fn is_dirty(&self) -> bool {
        !self.edits.is_empty()
    }

    /// Drops every unsaved edit.
    pub fn revert(&mut self) {
        self.edits.clear();
    }

    /// The notes under the fields this run's command line overrides.
    pub fn notes(&self) -> Vec<OverrideNote> {
        override_notes(&self.saved, &self.effective(&self.saved))
    }

    /// Checks every edited value, writes the edits to the file and applies the saved
    /// configuration, with this run's command line over it, telling `listener`; a value
    /// that does not check out refuses the Save, and nothing is written or applied.
    pub fn save(&mut self, listener: StatusListener) -> Result<(), Refusal> {
        for (field, value) in &self.edits {
            validate(field, value, &self.home).map_err(|message| Refusal {
                field: Some(field),
                message,
            })?;
        }
        let mut document = self.document.clone();
        for (field, value) in &self.edits {
            document.set(field.section, field.key, to_item(value));
        }
        let saved = document.to_config().map_err(unattributed)?;
        document.save(&self.path).map_err(unattributed)?;
        self.document = document;
        self.saved = saved;
        self.edits.clear();
        self.parts
            .applier
            .apply(self.effective(&self.saved), listener);
        Ok(())
    }

    fn effective(&self, file: &Config) -> Config {
        let mut effective = file.clone();
        (self.parts.overlay)(&mut effective);
        effective
    }
}

fn unattributed(error: anyhow::Error) -> Refusal {
    Refusal {
        field: None,
        message: format!("{error:#}"),
    }
}

/// What the window's status line says about `status`.
pub fn status_text(status: &ApplyStatus) -> String {
    match status {
        ApplyStatus::WaitsForDictation => "Applying after the current dictation…".to_owned(),
        ApplyStatus::Applied => "Applied".to_owned(),
        ApplyStatus::Rejected(reason) => format!("Saved, but not applied: {reason}"),
    }
}
