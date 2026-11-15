## Why

The user says "энтер" (often twice, "энтер энтер") at the end of a dictation to end it and press Return, and too often nothing happens; the user is fed up.

Evidence from the command history (`~/.voicepuncher/command-history`, 2026-11-13..15): about 225 utterances ended with an enter-like word and pressed Enter, but at least 7 did not. In each miss the Russian streaming recognizer glued the spoken "энтер энтер" (or a cut "энтер") into a new word that the exact list of mishearings does not hold:

- `энтеррендер` (2026-11-13T16-37-58)
- `интернтор` (2026-11-14T19-12-17)
- `антернтер` (2026-11-15T14-31-06)
- `китрентор` (2026-11-13T08-59-12)
- `интернета` (2026-11-15T14-48-07)
- `правильно интер` (2026-11-13T12-02-16)
- `вот энт` (2026-11-13T11-37-49)

`voice-commands/src/lib.rs` keeps an exact list of glued forms (`интернтер`, `энтеррентор`, `интеррендер`, `энтернтер`…). Every new session brings a new glued word, so the list never catches up.

Also, in 2026-11-15T14-48-07 the English recognizer heard "privileged enter and", yet no command was reported: "enter" was not the last word because a short filler followed it.

The rest of the pipeline is sound: every command a recognizer reported was acted on by the app, and the "empty" history entries are background speech, not lost commands.

## What Changes

- Each glued or cut "enter" the Russian recognizer wrote in the user's recent command history becomes an alias row of the voice commands, so it presses Enter: `энтеррендер`, `интернтор`, `антернтер`, `китрентор` read as "enter enter"; `энт` reads as one "enter".
- The user chose aliases over a fuzzy sound match: whatever word the recognizer glues is the command (2026-11-15).
- The new words join the test of every glued or mangled enter from the history.

Current stage: PoC. Target stage: PoC.
Scope: the alias rows of "press enter" in the voice commands.
Acceptance bar: the five words read as their enters; the ordinary words test stays green; `just check` passes.
Out of scope: `интернета` (an ordinary Russian word: an alias would press Enter in plain speech about the internet); "правильно интер" (the word already reads as enter, and a short sentence ending in it stays speech by the placement rules); the English "enter and" (a trailing filler); a fuzzy sound match; other commands; the English recognizer's speed.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: five more glued or cut enters from the user's history count as the command.

## Impact

- `voice-commands/src/lib.rs` (alias rows), `command-spotting/src/streaming/respell/tests.rs` (the history words test)

## Open points

- none: scope settled at explore (no gates picked).

## Release

SemVer: PATCH — bug fix.
Target version: 0.49.1.
