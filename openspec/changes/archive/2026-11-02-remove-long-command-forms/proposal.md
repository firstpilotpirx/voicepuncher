## Why

The user says commands by their one word ("start", "enter") or that word twice ("start start"); the long phrases ("start dictation", "press enter", "clear all text") are never said, yet they cost keyword spotting lines, mishearing rows, matcher rules and dozens of tests, and "cancel dictation" in particular is unwanted. Fewer forms also leave fewer ways for ordinary speech to be taken as a command.

## What Changes

- **BREAKING** Only the one-word form of each command and its doubled form count: "start", "stop", "cancel", "restart", "enter", "clear", "paste", each also said twice in a row. "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text", "clear text" and "paste text" are ordinary speech.
- The mishearings of the long forms go: "presenter", "star dictation", "stuck dictation", "capsule dictation". The mishearings of short forms stay ("start" for a quiet "restart", "and restart" for "enter start").
- The keyword spotter loses its long-phrase keywords; it keeps the trigger keywords of the short words.
- The commands keep their names in the protocol, the code and the logs ("start dictation" names the command started by "start"); renaming them is no behavior change and is left out.
- Every test, scenario and spec scenario that says a long form is deleted, or says the short form instead when the behavior it proves still holds.

## Capabilities

### New Capabilities

### Modified Capabilities
- `voice-control`: the forms a command may be said in; every requirement whose scenarios say a long form.

## Impact

- `voice-commands/src/lib.rs` (forms, mishearings), `command-spotting/` (keywords.txt, matcher, keyword spotting, tests), `app/` and `e2e-test/` voice-control tests and features, `docs/ubiquitous-language.md`.
- Stage: PoC, stays PoC. Acceptance bar: `just full-check` passes; no test or scenario says a long form as a command.
- Out of scope: renaming the command identifiers; the glued "enter enter" mishearings (`recognize-glued-enter-mishearings`); thread priorities.
- SemVer: MINOR (breaking pre-1.0), target 0.45.0.
