//! The engine lock (`app/packaging/engines.lock`): `[section]` headers and `key = "value"` lines, a key quoted
//! or bare; comments and blank lines skipped.

use std::collections::BTreeMap;

pub type Table = BTreeMap<String, String>;

#[derive(Debug, Default)]
pub struct EngineLock {
    pub sections: BTreeMap<String, Table>,
}

impl EngineLock {
    pub fn parse(text: &str, name: &str) -> Result<Self, String> {
        let mut sections: BTreeMap<String, Table> = BTreeMap::new();
        let mut section = String::new();
        for (number, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(header) = trimmed
                .strip_prefix('[')
                .and_then(|rest| rest.split(']').next())
            {
                section = header.to_string();
                continue;
            }
            let (key, value) = pair(line)
                .ok_or_else(|| format!("{name} line {}: not a key = \"value\" line", number + 1))?;
            sections
                .entry(section.clone())
                .or_default()
                .insert(key, value);
        }
        Ok(Self { sections })
    }

    /// A section, or the refusal naming it.
    pub fn section(&self, name: &str, lock: &str) -> Result<&Table, String> {
        self.sections
            .get(name)
            .ok_or_else(|| format!("{lock} has no [{name}]"))
    }

    /// A section that may be missing.
    pub fn optional(&self, name: &str) -> Table {
        self.sections.get(name).cloned().unwrap_or_default()
    }
}

/// `"key" = "value"` or `key = "value"`.
fn pair(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim().trim_matches('"').trim();
    let value = value.trim().strip_prefix('"')?;
    let value = &value[..value.find('"')?];
    (!key.is_empty()).then(|| (key.to_string(), value.to_string()))
}

/// A field of a section, empty when missing.
pub fn field<'a>(table: &'a Table, key: &str) -> &'a str {
    table.get(key).map_or("", String::as_str)
}
