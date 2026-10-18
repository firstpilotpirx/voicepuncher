## Why

The change speed-up-voice-start (released as v0.18.2) made voice starts give their start cue within 0.7 s of the command's words in at least 9 of 10 starts for every placement except one: the voice chain "enter start" said from standby and followed at once by speech. That chain gives its start cue at about 785 ms, over the 700 ms bar, so the E2E scenario "A chain ending in start gives its cue at once" in `e2e-test/tests/features/voice_control.feature` is tagged `@wip`.

Evidence (archived change `openspec/changes/archive/2026-10-18-speed-up-voice-start`, design.md § Measurements): the early checks of the command spotting service — the 1.0 s early search of the utterance start, both spotted checks and the windows — hear the synthesized "Enter" as "and a" ("and a star to come"). Only the head check at 1.5 s hears "Enter start coffee machine", so the chain is confirmed late and the cue comes late.

## What Changes

- The command spotting service hears "enter" reliably in a short clip, so the chain "enter start" said from standby and followed at once by speech gives its start cue within 0.7 s of the command's words, like every other placement.
- The E2E scenario "A chain ending in start gives its cue at once" loses its `@wip` tag and passes.

Current stage: PoC. Target stage: PoC.
Scope: recognition of "enter" by the early checks of the command spotting service when it begins the chain "enter start".
Acceptance bar: the scenario "A chain ending in start gives its cue at once" runs without `@wip` and passes, with the start cue within 0.7 s in at least 9 of 10 starts; every other voice control scenario stays green.
Out of scope: the other placements of the start command, which already meet the bar; other commands and chains; the dictation model.

## Capabilities

### Modified Capabilities

- `voice-control`: the chain "enter start" from standby gives its start cue within 0.7 s, as every other start placement does.

## Impact

- `command-spotting/` (keyword list `command-spotting/keywords.txt`, and the early checks if needed).
- `e2e-test/tests/features/voice_control.feature` (remove `@wip` from the scenario).

## Open points

- How to make "enter" heard in a short clip: proposed a trigger-only keyword for "enter" in `command-spotting/keywords.txt`, like the existing `@start_trigger` and `@restart_trigger`; alternatives (a prompt or hotword bias for the command speech recognition service) are left to design.
- Whether the fix must also hold for the user's own voice recordings in `~/.voicepuncher/voice-samples/`, not only synthesized speech: proposed yes, when such recordings of "enter start" exist.
- Target version: proposed 0.18.3 (PATCH above v0.18.2); take the next version above `main` if it moved.

## Release

SemVer: PATCH — a bug fix: one start placement misses the existing 0.7 s cue bar.
Target version: 0.18.3.

## Closed without implementation

Explored on 2026-10-18 and closed by the user's choice. The synthesized voice says "Enter" so that it sounds like "and a": the dictation model hears "and a start" as well, and the command model hears "and a", "and to" or "Enter" depending on the clip length, so the early checks are asked in time but cannot hear the word. A trigger-only keyword for "enter" would only ask a check that already runs. A command prompt in the early checks heard "and then start the engine" as "Enter start dictation." (a false Return); taking "and a" before "start" as "enter" was offered and declined. "enter start" from standby keeps its cue at about 0.79 s, and its E2E scenario stays `@wip`.
