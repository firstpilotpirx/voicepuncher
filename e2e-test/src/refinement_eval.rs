//! Refinement evaluation: dictation-style transcripts sent through the app's own
//! refine-or-raw path, scored against their expected normalization per category.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use voicepuncher::core::application::text_refiner::refine_or_raw;
use voicepuncher::core::application::TextRefiner;
use voicepuncher::core::domain::RawTranscript;

use crate::wer::{edit_distance_rate, wer};

/// Word error rate against the expected normalization above which a dictated command
/// counts as carried out.
const COMMAND_MAX_WER: f64 = 0.25;
/// Move of the Cyrillic share of letters, from the expected text to the pasted one, above
/// which the language counts as changed.
const MAX_CYRILLIC_SHIFT: f64 = 0.5;
const COMMANDS_CATEGORY: &str = "commands";
const OVERALL: &str = "all";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EvalItem {
    pub id: String,
    pub category: String,
    pub transcript: String,
    pub expected: String,
    /// Russian loanwords that must reach the paste in Cyrillic, each as the stem the
    /// pasted text must hold, compared ignoring case.
    #[serde(default)]
    pub keep_cyrillic: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    CommandCarriedOut { wer: f64 },
    LanguageChanged { from_cyrillic: f64, to_cyrillic: f64 },
    LoanwordLeftCyrillic { word: String },
}

#[derive(Debug, Clone)]
pub struct ItemResult {
    pub item: EvalItem,
    pub pasted: String,
    /// 1 − character error rate against the expected text, case and punctuation included.
    pub score: f64,
    /// The same score for the raw transcript pasted as is.
    pub raw_score: f64,
    pub verdict: Option<Verdict>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CategoryScore {
    pub category: String,
    pub score: f64,
    pub raw_score: f64,
    pub items: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub results: Vec<ItemResult>,
}

pub fn default_eval_set() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("refinement/eval_set.jsonl")
}

pub fn load_eval_set(path: &Path) -> Result<Vec<EvalItem>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the evaluation set {}", path.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("{} line {}: not an item", path.display(), index + 1))
        })
        .collect()
}

/// A report run once, by its first caller, and shared by every caller after it: the scenarios
/// that judge the whole evaluation set read one run instead of sending the set each.
pub struct SharedReport(std::sync::OnceLock<std::result::Result<Report, String>>);

impl SharedReport {
    pub const fn new() -> Self {
        Self(std::sync::OnceLock::new())
    }

    /// The report `run` made for the first caller; its failure for every caller when it failed.
    pub fn get_or_run(&self, run: impl FnOnce() -> Result<Report>) -> Result<Report> {
        self.0
            .get_or_init(|| run().map_err(|e| format!("{e:#}")))
            .clone()
            .map_err(anyhow::Error::msg)
    }
}

impl Default for SharedReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Sends every item through refine-or-raw, exactly as the app does before pasting.
pub fn run(refiner: &mut dyn TextRefiner, items: &[EvalItem]) -> Result<Report> {
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let raw = RawTranscript::new(item.transcript.clone())
            .with_context(|| format!("{}: empty transcript", item.id))?;
        let pasted = refine_or_raw(refiner, &raw)
            .map(|refined| refined.text.as_str().to_owned())
            .unwrap_or_default();
        results.push(judge(item, &pasted));
    }
    Ok(Report { results })
}

pub fn judge(item: &EvalItem, pasted: &str) -> ItemResult {
    ItemResult {
        item: item.clone(),
        pasted: pasted.to_owned(),
        score: exact_text_score(&item.expected, pasted),
        raw_score: exact_text_score(&item.expected, &item.transcript),
        verdict: verdict(item, pasted),
    }
}

fn verdict(item: &EvalItem, pasted: &str) -> Option<Verdict> {
    let from_cyrillic = cyrillic_share(&item.expected);
    let to_cyrillic = cyrillic_share(pasted);
    if (from_cyrillic - to_cyrillic).abs() > MAX_CYRILLIC_SHIFT {
        return Some(Verdict::LanguageChanged { from_cyrillic, to_cyrillic });
    }
    let lowercase = pasted.to_lowercase();
    if let Some(word) = item
        .keep_cyrillic
        .iter()
        .find(|word| !lowercase.contains(&word.to_lowercase()))
    {
        return Some(Verdict::LoanwordLeftCyrillic { word: word.clone() });
    }
    let word_error_rate = wer(&item.expected, pasted);
    (item.category == COMMANDS_CATEGORY && word_error_rate > COMMAND_MAX_WER)
        .then_some(Verdict::CommandCarriedOut { wer: word_error_rate })
}

/// 1 − character error rate with case, punctuation and single spaces kept.
fn exact_text_score(expected: &str, actual: &str) -> f64 {
    let chars = |text: &str| -> Vec<char> {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .collect()
    };
    (1.0 - edit_distance_rate(&chars(expected), &chars(actual))).max(0.0)
}

fn cyrillic_share(text: &str) -> f64 {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return 0.0;
    }
    let cyrillic = letters
        .iter()
        .filter(|c| matches!(c, '\u{0400}'..='\u{04FF}'))
        .count();
    cyrillic as f64 / letters.len() as f64
}

impl Report {
    /// Mean scores per category, sorted by category name.
    pub fn category_scores(&self) -> Vec<CategoryScore> {
        let mut categories: Vec<&str> = self
            .results
            .iter()
            .map(|r| r.item.category.as_str())
            .collect();
        categories.sort_unstable();
        categories.dedup();
        categories
            .into_iter()
            .map(|category| {
                let members: Vec<&ItemResult> = self
                    .results
                    .iter()
                    .filter(|r| r.item.category == category)
                    .collect();
                mean_score(category, &members)
            })
            .collect()
    }

    pub fn overall(&self) -> CategoryScore {
        mean_score(OVERALL, &self.results.iter().collect::<Vec<_>>())
    }

    pub fn failures(&self) -> Vec<&ItemResult> {
        self.results
            .iter()
            .filter(|r| r.verdict.is_some())
            .collect()
    }

    /// Per-category scores next to the raw-transcript baseline, then every failure.
    pub fn table(&self) -> String {
        let mut table = format!(
            "  {:<12} {:>6} {:>6} {:>5}\n",
            "category", "score", "raw", "items"
        );
        for row in self.category_scores().iter().chain([&self.overall()]) {
            let _ = writeln!(
                table,
                "  {:<12} {:>6.3} {:>6.3} {:>5}",
                row.category, row.score, row.raw_score, row.items
            );
        }
        for failure in self.failures() {
            let _ = writeln!(
                table,
                "  FAIL {} {:?}: {:?} -> {:?}",
                failure.item.id, failure.verdict, failure.item.transcript, failure.pasted
            );
        }
        table
    }
}

fn mean_score(category: &str, results: &[&ItemResult]) -> CategoryScore {
    let mean = |value: fn(&ItemResult) -> f64| {
        results.iter().map(|r| value(r)).sum::<f64>() / results.len().max(1) as f64
    };
    CategoryScore {
        category: category.to_owned(),
        score: mean(|r| r.score),
        raw_score: mean(|r| r.raw_score),
        items: results.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(category: &str, transcript: &str, expected: &str) -> EvalItem {
        EvalItem {
            id: format!("{category}-01"),
            category: category.to_owned(),
            transcript: transcript.to_owned(),
            expected: expected.to_owned(),
            keep_cyrillic: Vec::new(),
        }
    }

    fn report(results: &[(&EvalItem, &str)]) -> Report {
        Report {
            results: results
                .iter()
                .map(|(item, pasted)| judge(item, pasted))
                .collect(),
        }
    }

    #[test]
    fn should_score_case_and_punctuation() {
        // Given
        let it = item("punctuation", "привет как дела", "Привет, как дела?");
        // When
        let result = judge(&it, "Привет, как дела?");
        // Then
        assert_eq!(result.score, 1.0);
        assert!(result.raw_score < 0.9, "{}", result.raw_score);
    }

    #[test]
    fn should_flag_command_answered_instead_of_kept() {
        // Given
        let it = item(
            "commands",
            "сколько будет два плюс два",
            "Сколько будет два плюс два?",
        );
        // When
        let result = judge(
            &it,
            "Сколько будет два плюс два? Четыре, это равно четырём.",
        );
        // Then
        assert!(
            matches!(result.verdict, Some(Verdict::CommandCarriedOut { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn should_not_flag_command_pasted_as_dictated() {
        let it = item(
            "commands",
            "сколько будет два плюс два",
            "Сколько будет два плюс два?",
        );
        assert_eq!(judge(&it, "сколько будет два плюс два").verdict, None);
    }

    #[test]
    fn should_flag_language_change_in_any_category() {
        // Given
        let it = item("punctuation", "привет как дела", "Привет, как дела?");
        // When
        let result = judge(&it, "Hello, how are you?");
        // Then
        assert!(
            matches!(result.verdict, Some(Verdict::LanguageChanged { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn should_average_scores_per_category_and_overall() {
        // Given
        let a = item("terms", "докер", "Docker.");
        let b = item("fillers", "э-э да", "Да.");
        // When
        let report = report(&[(&a, "Docker."), (&b, "э-э да")]);
        // Then
        let categories = report.category_scores();
        assert_eq!(
            categories
                .iter()
                .map(|c| c.category.as_str())
                .collect::<Vec<_>>(),
            ["fillers", "terms"]
        );
        assert_eq!(categories[1].score, 1.0);
        assert_eq!(report.overall().items, 2);
        assert!(report.overall().score < 1.0);
    }

    #[test]
    fn should_list_failures_and_name_them_in_the_table() {
        // Given
        let bad = item(
            "commands",
            "переведи на английский",
            "Переведи на английский.",
        );
        let good = item("terms", "докер", "Docker.");
        // When
        let report = report(&[(&bad, "Translate into English."), (&good, "Docker.")]);
        // Then
        assert_eq!(report.failures().len(), 1);
        let table = report.table();
        for text in ["commands", "terms", "all", "commands-01"] {
            assert!(table.contains(text), "table lacks {text}: {table}");
        }
    }

    fn with_keep_cyrillic(mut it: EvalItem, words: &[&str]) -> EvalItem {
        it.keep_cyrillic = words.iter().map(|w| (*w).to_owned()).collect();
        it
    }

    #[test]
    fn should_flag_loanword_respelled_in_latin() {
        // Given
        let it = with_keep_cyrillic(
            item(
                "english",
                "менеджер попросил перезагрузить компьютер",
                "Менеджер попросил перезагрузить компьютер.",
            ),
            &["менеджер", "компьютер"],
        );
        // When
        let result = judge(&it, "Менеджер попросил перезагрузить computer.");
        // Then
        assert_eq!(
            result.verdict,
            Some(Verdict::LoanwordLeftCyrillic { word: "компьютер".to_owned() })
        );
        assert!(
            report(&[(&it, "Менеджер попросил перезагрузить computer.")])
                .table()
                .contains("компьютер")
        );
    }

    #[test]
    fn should_accept_loanword_kept_with_another_ending() {
        // Given
        let it = with_keep_cyrillic(
            item(
                "english",
                "мы деплоим релиз и закомитим фикс",
                "Мы деплоим релиз и закомитим фикс.",
            ),
            &["деплои", "закомит"],
        );
        // When
        let result = judge(&it, "Мы деплоили релиз и Закомитили фикс.");
        // Then
        assert_eq!(result.verdict, None);
    }

    #[test]
    fn should_load_the_committed_evaluation_set() {
        // Given / When
        let items = load_eval_set(&default_eval_set()).unwrap();
        // Then
        assert!(items.len() >= 40, "{}", items.len());
        assert!(
            items
                .iter()
                .filter(|i| i.category == "english")
                .filter(|i| !i.keep_cyrillic.is_empty())
                .count()
                >= 3,
            "fewer than 3 loanword controls"
        );
        for category in [
            "commands",
            "punctuation",
            "terms",
            "fillers",
            "mixed",
            "endings",
            "english",
        ] {
            assert!(
                items.iter().any(|i| i.category == category),
                "no {category} items"
            );
        }
    }

    #[test]
    fn should_run_a_shared_report_once_for_every_caller() {
        static SHARED: SharedReport = SharedReport::new();
        let runs = std::sync::atomic::AtomicU32::new(0);
        let expected = report(&[(&item("fillers", "ну это", "Это."), "Это.")]);
        let run = || {
            runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(expected.clone())
        };
        let first = SHARED.get_or_run(run).unwrap();
        let second = SHARED.get_or_run(run).unwrap();
        let third = SHARED.get_or_run(run).unwrap();
        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 1);
        for shared in [first, second, third] {
            assert_eq!(shared.table(), expected.table());
        }
    }

    #[test]
    fn should_give_every_caller_the_failure_of_the_one_run() {
        static SHARED: SharedReport = SharedReport::new();
        let runs = std::sync::atomic::AtomicU32::new(0);
        let run = || -> Result<Report> {
            runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            anyhow::bail!("the service did not answer")
        };
        for _ in 0..2 {
            let error = SHARED.get_or_run(run).unwrap_err();
            assert!(
                error.to_string().contains("the service did not answer"),
                "{error}"
            );
        }
        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
