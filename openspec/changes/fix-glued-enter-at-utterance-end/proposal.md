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

- An "enter" that the Russian recognizer glues or cuts at the end of an utterance during a dictation presses Enter: the match goes by sound shape (for example a last word close by edit distance to "энтер" or "энтер энтер", or starting with энт/инт/ант), not by an exact list.
- An English "enter" followed by a short filler word ("and", "uh") at the end of an utterance still counts as enter.
- The seven tails above become test cases, from the user's own recordings in the command history.

Current stage: PoC. Target stage: PoC.
Scope: spotting of "press enter" at the end of an utterance during a dictation (Russian and English recognizers).
Acceptance bar: the seven recorded misses above press Enter; no new false Enter across the user's recordings of ordinary dictation in the command history.
Out of scope: other commands; the speed of the English recognizer; false triggers outside the end of an utterance.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: "press enter" at the end of an utterance matches glued and cut forms by sound shape and tolerates a short trailing filler in English.

## Impact

- `voice-commands/src/lib.rs` (forms and mishearings), the matching in `command-spotting/`
- tests over the user's recordings (`needs_voice_samples`) and voice control scenarios

## Open points

- How loose the sound-shape match may be: the user says misses matter more than false triggers inside dictation; proposed: tune on the command history until the seven misses pass and ordinary dictation tails ("интересно", "интернет" mid-sentence, "агент") stay unmatched.
- Whether "интернета" at the very end should count; proposed: yes, only as the last word of an utterance during a dictation.

## Release

SemVer: PATCH — bug fix.
Target version: 0.49.1.
