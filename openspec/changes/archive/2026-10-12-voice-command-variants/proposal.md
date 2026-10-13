# Proposal

## Why

In the user's manual runs voice control mostly failed: on 40 recordings of the user's own voice the keyword spotting model found 24, and "start dictation" only 2 of 9, whatever its thresholds. The user also wants to say "please" after a command ("press enter please") and the shorter "clear text" for "clear all text".

## What Changes

- Every command may end with "please": "start dictation please", "press enter please", and so on.
- "clear text" (with or without "please") clears the text, like "clear all text".
- The command spotting service also asks the speech recognition service (whisper-server) what was said in each short utterance (up to 3 s) and reports a command when the whole utterance is a command phrase, allowing small recognition slips ("star dictation", "presenter", "clear all texts"). A phrase found by the keyword spotter still counts too; the app gets one command per utterance.
- When the speech recognition service does not answer, the keyword spotter works alone, as before.
- A command said twice in one utterance ("press enter, press enter") counts once.
- Tests run on the user's own recordings when they exist on the machine (`~/.voicepuncher/voice-samples/`, never committed): every recorded command must be recognized exactly once, apart from recordings marked unclear (repeats without a half-second pause are one command).
- Long hands-free sessions (dictate, stop, press enter, cancel, clear, dictate again, with and without "please") run end to end in a synthesized voice and in the user's recorded voice.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `voice-control`: the five commands accept a trailing "please"; "clear text" is a phrase of "clear all text"; a command said twice in one utterance counts once; the user's own way of saying the commands is recognized; a whole hands-free session runs as said.

## Impact

- `command-spotting/`: keyword lines for the new phrases; whisper-backed utterance check (HTTP client to the speech recognition service, `--transcription-url`, `--max-command-secs`); a short pre-roll buffer so the utterance sent to whisper keeps its first sound; a pure phrase matcher with fuzzy matching.
- `app/`: the recognizer counts several spots of the same command in one utterance as one; "start dictation" switches the phrase set to the dictation phrases before the recording starts, so "stop dictation" said right after is heard.
- Tests: unit (matcher, recognizer, pipeline), service integration (keywords for the new phrases, keyword spotter alone when whisper does not answer), adapter integration on the user's recordings, E2E for the new phrases and short Russian phrases with `say`, E2E hands-free sessions in a synthesized and the user's recorded voice.
- Docs: README voice control table, glossary.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** every command the user said in `~/.voicepuncher/voice-samples/` is recognized exactly once, apart from recordings listed as unclear in their manifest (repeats without a half-second pause are one command); the three hands-free sessions run as said; no command on the benchmark corpus; `just check` and `just full-check` (bar the known chunk-seam failure) pass; the user's manual run confirms.
- **Out of scope:** commands in Russian; training or fine-tuning a model; other new commands.

## Release

MINOR — new phrases of an existing capability; target version **0.8.0**.
