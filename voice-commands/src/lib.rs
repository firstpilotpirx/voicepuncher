//! The voice commands, declared once: their phrases, the ways to say them, the states that
//! accept them, and the ways the speech recognition service mishears them. The command
//! spotting service matches transcripts against these; the app's tests check that its own
//! lists agree.

pub const START: &str = "start dictation";
pub const STOP: &str = "stop dictation";
pub const CANCEL: &str = "cancel dictation";
pub const RESTART: &str = "restart dictation";
pub const ENTER: &str = "press enter";
pub const CLEAR: &str = "clear all text";
pub const PASTE: &str = "paste text";

/// Every command phrase.
pub const COMMANDS: [&str; 7] = [START, STOP, CANCEL, RESTART, ENTER, CLEAR, PASTE];

/// The phrases each state accepts; a dictation runs or it does not.
pub const STANDBY: &[&str] = &[START, ENTER, CLEAR, PASTE];
/// "enter" during a dictation ends it and presses Return.
pub const DICTATING: &[&str] = &[STOP, CANCEL, RESTART, ENTER];

/// The phrases accepted after `phrase` was carried out.
pub fn accepted_after(phrase: &str) -> &'static [&'static str] {
    if phrase == START || phrase == RESTART {
        DICTATING
    } else {
        STANDBY
    }
}

/// One way to say a command: its phrase and the words said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form {
    pub phrase: &'static str,
    pub words: &'static [&'static str],
}

const fn form(phrase: &'static str, words: &'static [&'static str]) -> Form {
    Form { phrase, words }
}

/// The ways people say each command, longer forms first, so "stop dictation" wins over "stop".
pub const FORMS: &[Form] = &[
    form(CLEAR, &["clear", "all", "text"]),
    form(START, &["start", "dictation"]),
    form(STOP, &["stop", "dictation"]),
    form(CANCEL, &["cancel", "dictation"]),
    form(RESTART, &["restart", "dictation"]),
    form(ENTER, &["press", "enter"]),
    form(CLEAR, &["clear", "text"]),
    form(PASTE, &["paste", "text"]),
    form(START, &["start"]),
    form(STOP, &["stop"]),
    form(RESTART, &["restart"]),
    form(ENTER, &["enter"]),
    form(CLEAR, &["clear"]),
    form(PASTE, &["paste"]),
    form(CANCEL, &["cancel"]),
];

/// How a mishearing stands for its commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MishearingKind {
    /// The words are heard in place of the whole command, wherever it is said.
    Alias,
    /// A short sentence ending in the words is heard for a quiet short form of the command;
    /// a longer sentence ending in them is speech.
    NearMiss,
    /// The words close a chain of commands after another command.
    AfterCommand,
}

/// The user's recording that produced a mishearing: a session of
/// `~/.voicepuncher/voice-samples/` and the second the command starts at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recording {
    pub session: &'static str,
    pub at_secs: f32,
}

/// Words the speech recognition service hears in place of commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mishearing {
    pub heard: &'static [&'static str],
    /// The commands the words stand for, in the order said.
    pub means: &'static [&'static str],
    pub kind: MishearingKind,
    pub recording: Option<Recording>,
}

const fn mishearing(
    heard: &'static [&'static str],
    means: &'static [&'static str],
    kind: MishearingKind,
) -> Mishearing {
    Mishearing { heard, means, kind, recording: None }
}

/// The mishearings met so far: one row each, never a branch in the matcher.
pub const MISHEARINGS: &[Mishearing] = &[
    // "press enter" runs together.
    mishearing(&["presenter"], &[ENTER], MishearingKind::Alias),
    mishearing(&["star", "dictation"], &[START], MishearingKind::Alias),
    mishearing(&["stuck", "dictation"], &[START], MishearingKind::Alias),
    mishearing(&["capsule", "dictation"], &[CANCEL], MishearingKind::Alias),
    // The user's quiet "restart" comes out as "Let's start." or "Start.".
    Mishearing {
        heard: &["start"],
        means: &[RESTART],
        kind: MishearingKind::NearMiss,
        recording: Some(Recording { session: "session-2", at_secs: 27.23 }),
    },
    // "enter start" said quickly.
    mishearing(
        &["and", "restart"],
        &[ENTER, START],
        MishearingKind::AfterCommand,
    ),
];

/// A word that may close a command and is still being said when a clip cuts it: the speech
/// heard up to `cut_secs` after the clip may be the rest of the word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailingWord {
    pub word: &'static str,
    pub cut_secs: f32,
}

/// The words said after a command out of politeness; they do not change it.
pub const TRAILING_WORDS: &[TrailingWord] = &[TrailingWord { word: "please", cut_secs: 0.25 }];

/// The trailing word `word` is, if any.
pub fn trailing(word: &str) -> Option<&'static TrailingWord> {
    TRAILING_WORDS.iter().find(|t| t.word == word)
}

#[cfg(test)]
mod tests;
