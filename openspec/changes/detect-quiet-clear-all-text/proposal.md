# Proposal

## Why

"clear all text" said in a quiet voice does nothing. Scaled so its loudest 10 ms has an RMS of 0.005 (the level of the user's quietest recorded commands), `say`'s "clear all text" gives no speech segment in the command spotting service: no confirmation is asked and no command is carried out. The speech recognition service hears the same quiet clip right ("Clear all text."), and a quiet "clear" alone works. This breaks the spec promise that a command said on its own in a quiet voice takes effect. It fails on `main` as well as on the branch of `recognize-users-quiet-restart`, whose E2E row "A command said in a quiet voice is recognized | clear all text" found it.

Seen at the same time: `say`'s Daniel voice saying "start" at the quiet level is heard as "Stop." by today's confirmation clip, which would end a running dictation.

## What Changes

- A quiet "clear all text" said on its own is noticed and carried out while no dictation runs.
- Find out why the voice activity detector (or the keyword spotter) misses it, and fix that without new false commands.
- Check whether a quiet "start" heard as "Stop." during a dictation needs its own fix.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: the requirement already exists (`voice-control`, "The user's own way of saying the commands is recognized").

## Scope

- Current stage: PoC. Target stage: PoC.
- Acceptance bar: the E2E row "A command said in a quiet voice is recognized | clear all text" passes; every other voice control scenario, `voice_forms_it` and `voice_samples_it` still pass; `just full-check` shows no new failures.

## Out of scope

- A different voice activity, keyword spotting or speech recognition model.
- New command phrases.

## Release

PATCH (bug fix): 0.14.5, the next patch after 0.14.4 of `recognize-users-quiet-restart`, which lands first.

## Impact

- `command-spotting/src/engine.rs`, `command-spotting/src/engine/pause.rs`: speech detection of quiet utterances.
- `e2e-test/tests/features/voice_control.feature`: the quiet `clear all text` row, `@wip` until this lands.
