//! Decides which command phrase, if any, a transcript of a whole utterance is.

mod placed;

pub use placed::{closing_word_of_start, Found, Placement};

use voicepuncher_voice_commands::{trailing, Form, MishearingKind, FORMS, MISHEARINGS};

/// Where a transcript's sentences end.
const SENTENCE_ENDS: &[char] = &['.', ',', '!', '?', '\n'];
/// Words a transcript may hold inside a phrase without changing it ("cancel the quotation").
const FILLER_WORDS: &[&str] = &["the", "a", "an"];
/// The most words a transcript may hold to be a near miss: "Let's start." is "let", "s",
/// "start"; a longer sentence ending in the word is speech, not a misheard command.
const NEAR_MISS_WORDS: usize = 3;
/// Largest share of characters that may differ between a transcript and a phrase.
const MAX_DIFFERENCE: f32 = 0.25;
/// Largest share of characters that may differ within one word ("activation" for
/// "dictation"), so that "stop" never passes for "start".
const MAX_WORD_DIFFERENCE: f32 = 0.4;

/// The command whose words open speech: the start of an utterance is searched early for it
/// when the app does not say whether to.
const STARTED_EARLY: &str = "start dictation";

/// The selected command phrases and the ways people say them.
pub struct PhraseMatcher {
    phrases: Vec<String>,
    /// Whether the start of an utterance is searched early for a command opening speech.
    early_start: bool,
}

impl PhraseMatcher {
    /// The selected command phrases.
    fn selected(&self) -> &[String] {
        &self.phrases
    }

    /// Whether `phrase` is one of the selected command phrases.
    pub fn selects(&self, phrase: &str) -> bool {
        self.phrases.iter().any(|p| p == phrase)
    }

    pub fn new(phrases: &[String]) -> Self {
        let early_start = phrases.iter().any(|p| p == STARTED_EARLY);
        Self { phrases: phrases.to_vec(), early_start }
    }

    /// The same phrases, searched early at the start of an utterance as `early_start` says,
    /// or as `new` decided when it says nothing.
    #[must_use]
    pub fn with_early_start(self, early_start: Option<bool>) -> Self {
        Self { early_start: early_start.unwrap_or(self.early_start), ..self }
    }

    /// Whether the start of an utterance is searched early for a command opening speech.
    pub const fn starts_early(&self) -> bool {
        self.early_start
    }

    /// Selects the phrases of `self` and of `other`, each once, `self`'s first.
    pub fn combined_with(&self, other: &Self) -> Self {
        let mut phrases = self.phrases.clone();
        for phrase in other.selected() {
            if !phrases.contains(phrase) {
                phrases.push(phrase.clone());
            }
        }
        Self { phrases, early_start: self.early_start }
    }

    /// The command phrase the whole `transcript` says, if it says one of the selected. A
    /// transcript of several sentences counts when every sentence says the same phrase.
    pub fn phrase_in(&self, transcript: &str) -> Option<String> {
        self.phrase_in_sentence(transcript)
            .or_else(|| self.phrase_in_every_sentence(transcript))
    }

    /// The selected command `transcript` nearly says without saying it: a short sentence
    /// ending in a word the speech recognition service hears in place of a command's short
    /// form ("Let's start." for a quiet "restart").
    pub fn near_miss(&self, transcript: &str) -> Option<String> {
        let said = words(transcript);
        if said.len() > NEAR_MISS_WORDS {
            return None;
        }
        MISHEARINGS
            .iter()
            .filter(|m| m.kind == MishearingKind::NearMiss)
            .find(|m| ends_with(&said, m.heard) && self.selects(m.means[0]))
            .map(|m| m.means[0].to_owned())
    }

    fn phrase_in_every_sentence(&self, transcript: &str) -> Option<String> {
        let sentences: Vec<&str> = transcript
            .split(SENTENCE_ENDS)
            .filter(|s| !words(s).is_empty())
            .collect();
        let mut phrases = sentences.iter().map(|s| self.phrase_in_sentence(s));
        let first = phrases.next()??;
        phrases.all(|p| p.as_ref() == Some(&first)).then_some(first)
    }

    fn phrase_in_sentence(&self, sentence: &str) -> Option<String> {
        let said = once(&without_trailing_word(&words(sentence)));
        if said.is_empty() {
            return None;
        }
        let joined = said.join(" ");
        self.surface_forms()
            .filter(|(_, form)| close_word_by_word(&said, form))
            .filter(|(_, form)| difference(&joined, form) <= MAX_DIFFERENCE)
            .min_by(|(_, a), (_, b)| difference(&joined, a).total_cmp(&difference(&joined, b)))
            .map(|(phrase, _)| phrase)
    }

    /// Every way to say each selected phrase, with the phrase it stands for.
    fn surface_forms(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.phrases.iter().flat_map(|phrase| {
            forms()
                .filter(move |form| form.phrase == phrase)
                .map(move |form| (phrase.clone(), form.words.join(" ")))
        })
    }
}

/// Lowercase words of letters only, in any script, without filler words.
fn words(transcript: &str) -> Vec<String> {
    transcript
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty() && !FILLER_WORDS.contains(w))
        .map(str::to_owned)
        .collect()
}

/// Every way to say a command: its forms, then the words heard in place of it.
fn forms() -> impl Iterator<Item = Form> {
    let aliases = MISHEARINGS
        .iter()
        .filter(|m| m.kind == MishearingKind::Alias)
        .map(|m| Form { phrase: m.means[0], words: m.heard });
    FORMS.iter().copied().chain(aliases)
}

/// Whether `said` ends with the words `heard`.
fn ends_with(said: &[String], heard: &[&str]) -> bool {
    said.len() >= heard.len()
        && said[said.len() - heard.len()..]
            .iter()
            .zip(heard)
            .all(|(a, b)| a == b)
}

fn without_trailing_word(words: &[String]) -> Vec<String> {
    match words.split_last() {
        Some((last, rest)) if trailing(last).is_some() => rest.to_vec(),
        _ => words.to_vec(),
    }
}

/// The words said once when the utterance repeats them ("press enter, press enter").
fn once(words: &[String]) -> Vec<String> {
    let half = words.len() / 2;
    if words.len() % 2 == 0 && half > 0 && words[..half] == words[half..] {
        words[..half].to_vec()
    } else {
        words.to_vec()
    }
}

/// As many words as the phrase, each close to the phrase's word in its place.
fn close_word_by_word(said: &[String], form: &str) -> bool {
    let form: Vec<&str> = form.split(' ').collect();
    said.len() == form.len()
        && said
            .iter()
            .zip(form)
            .all(|(word, expected)| difference(word, expected) <= MAX_WORD_DIFFERENCE)
}

/// Character edit distance between `a` and `b`, as a share of the longer one.
fn difference(a: &str, b: &str) -> f32 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()] as f32 / a.len().max(b.len()).max(1) as f32
}

#[cfg(test)]
mod tests;
