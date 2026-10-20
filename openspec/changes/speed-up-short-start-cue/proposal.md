## Why

The E2E scenario "The user's recorded starts give their cue at once" (`e2e-test/tests/features/voice_control.feature`, `@seams`) requires 9 of 10 of the user's recorded starts to give the start cue within 0.7 s of the end of the command's words. Two recordings sit on that bar in every run since 2026-10-19, on `main` and on branches:

- recorded "start please" at 31.35 s, followed by speech: 765–803 ms (always over);
- recorded "start" at 7.22 s, alone: 676–733 ms (over about half the time).

The scenario "Restart said on its own gives its cue at once" has the same bar and sits near it too: synthesized "restart" alone took 580–637 ms, then 701 ms on 2026-10-19 (failed).

With 12 recordings (11 needed) the scenario fails roughly every other `just full-check`. The other starts give their cue in 180–610 ms. The user does not wait for the cue: the words said right after the command are kept by the always-running capture and belong to the dictation (the lead-in). The cost of a late cue is the flaky scenario and a thinner margin on the responsiveness it measures; the mechanism of the lead-in stays as it is.

## What Changes

- The start cue for a short "start" said alone, for "start please" followed by speech and for "restart" said alone comes within 0.7 s of the end of the words, with room to spare, like the other starts. Measured on 2026-10-20 (`just e2e-seams` with debug logs), the time goes to three causes in the command spotting engine, each removed by this change:
  - "start please" is a complete command: the early check of an utterance's start hears "Start please." whole but refuses it, since a start at the head needs a word after it and "please" counts as part of the command; the full check 0.5 s later accepts it. The early check and the first spotted check now accept a start whose command words end with "please" without a word after it.
  - A trigger spotted in a short segment whose hearing is still pending asks the second hearing (the steady clip) at once, beside the first, instead of after its answer: "restart" alone is first heard as "Restocked." and today waits two requests in a row.
  - While no dictation runs, a short utterance is heard once its quiet run reaches half the minimum silence; the answer counts only when the pause then reaches the minimum silence with no new sound, else it is dropped. "start" alone then gets its cue about 0.15 s sooner, with room for a slow confirmation.

Current stage: PoC. Target stage: PoC.
Scope: why these two recorded starts confirm late (for example waiting for the utterance end or a pause before confirming a single short word, or the segment check after the head check), and making them confirm sooner.
Acceptance bar: both scenarios pass 5 `just full-check` runs in a row; the 0.7 s bar and the 9-of-10 share stay as they are.
Out of scope: loosening the bar; other voice commands; the dictation path.

## Capabilities

### Modified Capabilities

- `voice-control`: the start cue of a short or polite "start" comes within the same bound as other starts.

## Impact

- `command-spotting/` engine (when checks of a start are asked and confirmed)
- `e2e-test/tests/features/voice_control.feature` (unchanged bar; the scenario becomes reliable)

## Open points

- Whether the delay sits in the command spotting engine or in the app's handling of the confirmation — settled on 2026-10-20: in the engine (the app delivers and cues within 50 ms); the three causes above.
- Target version — 0.24.4, the PATCH above match-late-confirmation-to-its-state (0.24.3), which this change waits for.

## Release

SemVer: PATCH — a faster start cue, no new capability.
Target version: 0.24.4.
