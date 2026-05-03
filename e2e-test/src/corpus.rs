use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, serde::Deserialize)]
pub struct Sample {
    pub id: String,
    pub path: PathBuf,
    pub reference: String,
    pub lang: String,
    pub duration_s: f32,
    pub source: String,
}

pub fn load_manifest(manifest_path: &Path) -> Result<Vec<Sample>> {
    let content = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read manifest at {manifest_path:?}"))?;

    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));

    let mut samples = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut sample: Sample = serde_json::from_str(line)
            .with_context(|| format!("failed to parse manifest line {}", line_no + 1))?;
        if sample.path.is_relative() {
            sample.path = manifest_dir.join(&sample.path);
        }
        samples.push(sample);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn should_parse_manifest_jsonl() {
        // Given
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"{{"id":"s1","path":"audio/s1.wav","reference":"hello world","lang":"en","duration_s":1.5,"source":"librispeech"}}"#
        ).unwrap();
        // When
        let samples = load_manifest(tmp.path()).unwrap();
        // Then
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].id, "s1");
        assert_eq!(samples[0].reference, "hello world");
        assert_eq!(samples[0].lang, "en");
    }

    #[test]
    fn should_resolve_relative_path_against_manifest_dir() {
        // Given
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"{{"id":"s1","path":"audio/s1.wav","reference":"test","lang":"ru","duration_s":2.0,"source":"fleurs"}}"#
        ).unwrap();
        // When
        let samples = load_manifest(tmp.path()).unwrap();
        // Then
        let expected_dir = tmp.path().parent().unwrap();
        assert!(samples[0].path.starts_with(expected_dir));
    }
}
