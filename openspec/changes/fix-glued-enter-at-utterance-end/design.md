## Context

The Russian streaming recognizer glues the user's "энтер энтер" into a new word, and cuts a lone "энтер". The respelling (`command-spotting/src/streaming/respell/`) reads a Cyrillic word as command sounds, and first looks it up in the alias rows of `voice-commands/src/lib.rs`. Seven misses in the command history of 2026-11-13..15; five of them are words neither the sounds nor the aliases read.

## Goals / Non-Goals

**Goals:** the five words press Enter. **Non-Goals:** a fuzzy match, the English trailing filler, the placement rules.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| How to catch new glued words | one alias row per word | the user chose it; one row each, never a branch in the matcher | fuzzy sound match: false triggers in ordinary words, a new design |
| `энтеррендер`, `интернтор`, `антернтер`, `китрентор` | `[ENTER, ENTER]` | each is "энтер энтер" glued; `китрентор` came after "кит", its tail is the double | one enter: the user said it twice |
| `энт` | `[ENTER]` | a lone cut "энтер" | none |
| `интернета` | no alias | an ordinary word; added to the ordinary words test | an alias: Enter in plain speech |

## Risks / Trade-offs

- An alias counts wherever it sits; the five words are no Russian words, so plain speech does not hold them. `энт` is the shortest; no Russian word is spelled so alone.

## Test levels

| Spec scenario | Level |
|---|---|
| A Russian-accented stop is heard as stop | unchanged: E2E `.feature` (`voice_control.feature`) |
| A Russian word that is no command stays speech | unchanged: E2E `.feature` (`voice_control.feature`) |
| A glued enter enter written by the Russian recognizer counts as the doubled command | unchanged: unit (`command-spotting/src/streaming/session/tests/russian_forms.rs`) |
| A mangled enter written by the Russian recognizer counts as enter | unchanged: unit (`russian_forms.rs`) |
| Every glued or mangled enter from the history reads as the command | unit (`command-spotting/src/streaming/respell/tests.rs`), the five new words added |
| An ordinary Russian word with the letters of enter stays speech | unit (`respell/tests.rs`), `интернета` added |
