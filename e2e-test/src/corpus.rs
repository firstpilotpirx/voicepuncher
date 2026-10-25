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
        .with_context(|| format!("failed to read manifest at {}", manifest_path.display()))?;

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

/// Picks at most `limit` samples, taking one per language in turn so every
/// language gets an equal share; a language that runs out leaves its slots to
/// the others. The result keeps manifest order.
pub fn select_balanced(samples: Vec<Sample>, limit: usize) -> Vec<Sample> {
    let mut queues: Vec<(String, std::collections::VecDeque<usize>)> = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        match queues.iter_mut().find(|(lang, _)| *lang == s.lang) {
            Some((_, q)) => q.push_back(i),
            None => queues.push((s.lang.clone(), [i].into())),
        }
    }
    let mut picked = vec![false; samples.len()];
    let mut remaining = limit.min(samples.len());
    while remaining > 0 {
        for (_, q) in &mut queues {
            if remaining == 0 {
                break;
            }
            if let Some(i) = q.pop_front() {
                picked[i] = true;
                remaining -= 1;
            }
        }
    }
    samples
        .into_iter()
        .zip(picked)
        .filter_map(|(s, p)| p.then_some(s))
        .collect()
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

    fn sample(id: &str, lang: &str) -> Sample {
        Sample {
            id: id.to_owned(),
            path: PathBuf::from(format!("audio/{id}.wav")),
            reference: String::new(),
            lang: lang.to_owned(),
            duration_s: 1.0,
            source: String::new(),
        }
    }

    fn ids(samples: &[Sample]) -> Vec<&str> {
        samples.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn should_take_equal_share_per_language_in_manifest_order() {
        // Given
        let samples = vec![
            sample("ru1", "ru"),
            sample("ru2", "ru"),
            sample("ru3", "ru"),
            sample("en1", "en"),
            sample("en2", "en"),
            sample("en3", "en"),
        ];
        // When
        let picked = select_balanced(samples, 4);
        // Then
        assert_eq!(ids(&picked), vec!["ru1", "ru2", "en1", "en2"]);
    }

    #[test]
    fn should_fill_remaining_slots_when_a_language_runs_out() {
        // Given
        let samples = vec![
            sample("ru1", "ru"),
            sample("en1", "en"),
            sample("en2", "en"),
            sample("en3", "en"),
        ];
        // When
        let picked = select_balanced(samples, 3);
        // Then
        assert_eq!(ids(&picked), vec!["ru1", "en1", "en2"]);
    }

    #[test]
    fn should_return_all_samples_when_limit_exceeds_corpus() {
        // Given
        let samples = vec![sample("ru1", "ru"), sample("en1", "en")];
        // When
        let picked = select_balanced(samples, 10);
        // Then
        assert_eq!(ids(&picked), vec!["ru1", "en1"]);
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
