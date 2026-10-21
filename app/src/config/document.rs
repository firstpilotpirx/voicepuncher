//! The configuration file as the user wrote it: comments, order and formatting kept, so the
//! settings window changes only the values the user edited.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, Table};

use super::Config;

/// Suffix of the file a save writes before it replaces the configuration file.
const TEMPORARY_SUFFIX: &str = "saving";

#[derive(Clone)]
pub struct ConfigDocument {
    document: DocumentMut,
}

impl ConfigDocument {
    pub fn parse(text: &str) -> Result<Self> {
        let document = text.parse::<DocumentMut>().context("parse config")?;
        Ok(Self { document })
    }

    /// The file at `path`, or an empty document when there is none yet.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).with_context(|| path.display().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::parse(""),
            Err(e) => Err(e).with_context(|| format!("read config: {}", path.display())),
        }
    }

    /// The value the file sets for `key` in `section`; none when the file leaves it out.
    pub fn get(&self, section: &str, key: &str) -> Option<&Item> {
        self.document
            .get(section)
            .and_then(|table| table.get(key))
            .filter(|item| !item.is_none())
    }

    /// Sets `key` in `section`, adding the section at the end when the file has none; the
    /// decoration of an existing value (a comment beside it) stays.
    pub fn set(&mut self, section: &str, key: &str, value: Item) {
        let table = self
            .document
            .entry(section)
            .or_insert_with(|| Item::Table(Table::new()));
        match (table.get_mut(key), value) {
            (Some(Item::Value(old)), Item::Value(mut new)) => {
                *new.decor_mut() = old.decor().clone();
                *old = new;
            }
            (_, value) => table[key] = value,
        }
    }

    /// The configuration the document holds, checked as the loader checks a file.
    pub fn to_config(&self) -> Result<Config> {
        super::parse(&self.document.to_string(), "the edited settings")
    }

    /// Replaces the file at `path` in one step: a reader sees the old file or the new one.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("create {}", folder.display()))?;
        }
        let temporary = path.with_extension(TEMPORARY_SUFFIX);
        std::fs::write(&temporary, self.document.to_string())
            .with_context(|| format!("write {}", temporary.display()))?;
        std::fs::rename(&temporary, path).with_context(|| format!("replace {}", path.display()))
    }
}

impl fmt::Display for ConfigDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.document)
    }
}

#[cfg(test)]
mod tests;
