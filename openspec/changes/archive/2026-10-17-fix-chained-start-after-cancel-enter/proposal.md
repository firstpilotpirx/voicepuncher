## Why

`voice_forms_it` turns red once the voice activity detector notices speech 0.1 s earlier, as `detect-quiet-clear-all-text` makes it do: the user's recorded "never mind cancel enter start" (session-4, 32.41 s, while dictating) carries out "cancel dictation" and "press enter" but loses "start dictation".

The command spotting service's log shows why. The check of the running utterance, cut at 3.50 s, hears "Nevermind, cancel, enter style." and reports "cancel" and "enter". The closed utterance is then heard whole, "Nevermind, cancel, enter, start.", but its chain starts before what was already reported, so all of it is dropped, "start" too. With `main`'s detector the check is cut 0.1 s later and hears all three.

This breaks the `voice-control` promise that a chain of commands is carried out in order, and blocks `detect-quiet-clear-all-text`.

## What Changes

- When the whole utterance is heard after a check already reported the first commands of a chain, the commands not reported yet are carried out, in order; none twice.
- The recorded "never mind cancel enter start" carries out all three commands, in order, with either detector.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: "A chain of commands is carried out in order" gains a scenario for a chain whose end is heard only once the utterance ends.

## Scope

- Current stage: PoC. Target stage: PoC.
- Acceptance bar: `voice_forms_it` passes with every recorded form, with `main`'s detector and with the one of `detect-quiet-clear-all-text`, nothing added to its `KNOWN_WRONG`; `just full-check` passes.

## Out of scope

- A different speech recognition model for commands, or new command phrases.

## Release

PATCH (bug fix): 0.15.2, or the next patch above `main` when it lands (`detect-quiet-clear-all-text` waits for this change).
