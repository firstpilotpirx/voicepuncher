## Context

Each command was declared with a long phrase ("stop dictation"), a one-word short form ("stop") and mishearings of the long phrase. Both recognizers read these forms from `voice-commands`: the streaming recognizer matches transcript words against `FORMS`; the keyword spotter had a keyword line per long phrase and per short word, and trigger keywords for "start", "stop", "cancel" and "restart". The app strips command words from dictated text with its own list (`COMMAND_FORMS` in `app/src/core/domain/command_words/`).

## Goals / Non-Goals

**Goals:**
- One way to say each command: its one word, or that word twice.
- Delete the code that only served long forms, not just the data.

**Non-Goals:**
- Renaming `VoiceCommand` variants or the protocol's command names.
- The glued "enter enter" mishearings, thread priorities.

## Decisions

- **Data first.** `FORMS` holds one one-word form per command; the long-form mishearing rows and the `Alias` mishearing kind go. The streaming matcher then needs no alias lookup, no filler-word filter and no "two-word form reported at once" path; those are removed. Alternative: keep the long forms behind a flag — rejected, nothing uses them.
- **Keyword spotter.** Its phrase stream keeps lines for "enter", "clear" and "paste"; "start", "stop", "cancel" and "restart" are heard only through their trigger keywords, as a phrase line beside a trigger for the same word would double-count it. `keywords::select` counts a phrase served by a trigger as known; when only triggers serve the selected phrases, no phrase stream is created.
- **Placed commands.** `command_at` picks the closest form at a position, so "restart" is never read as "start" now that both are one-word forms.
- **Dictated text.** `COMMAND_FORMS`, `START_FORMS`, `RESTART_FORMS` and `PASTE_FORMS` keep only one-word and Cyrillic forms: "stop dictation" ending a text is kept as speech.
- **Tests.** A test that proves only a long form is deleted; one whose behavior still holds says the one word instead. Requirement headings keep the command identifiers, so the feature files' `Rule:` lines stay; "\"clear text\" erases the focused text too" is removed with its rule.

## Risks / Trade-offs

- [A user habit of the long forms] → the user asked for it; a long form now dictates its words.
- [Fewer keyword lines change spotting timing] → the E2E voice control scenarios and `full-check` measure it.
