## Why

The `full-check` of `refresh-human-docs` (worktree `sleepy-kapitsa-e5d9b5`, commit `f0e051d1`, 2026-10-22 16:28, log `run-visible/20261022-162801-sh--c.log`) failed in the stage `e2e-smoke-voice` (recipe `_e2e-smoke-part`, light lock class, lock wait 1.3 s, held 126 s) on the `@smoke @serial` scenario "Restart, a short pause, then speech that starts like another word" (`e2e-test/tests/features/voice_control.feature:495`, step `e2e-test/tests/steps/voice_control/session/speech_after_start.rs:217`): "no dictation started; events: [Listening, RecordingStarted]". The dictation ran, someone said "Restart", paused 0.4 s and said the sentence; the transcript on stderr shows what the service heard: "Stud, coffee machine on the third floor is broken." — the command word was swallowed into the next sentence as a one-syllable fragment, so no restart was confirmed and the scenario waited for a dictation that never started. The 6 other scenarios of the part passed. `refresh-human-docs` touches only documents, `app/tests/docs_check` and the stage-input table of the `justfile`, so the failure is not its own. Beside the run: `just test` of two other worktrees (`interesting-diffie-ec2773`, `practical-hofstadter-aa5b44`), both unlocked build stages; no other light or heavy phase held the machine lock. The same step family has been fixed before (`fix-short-pause-after-command` #82, `recognize-users-quiet-restart` #110, `make the flaky tests deterministic` #207), and "restart"→"Restocked." is one of the mishearings `command-spotting/src/engine/confirming.rs` handles by a branch; "Stud" is not among them.

## What Changes

- A "Restart" cut short by a short pause before the next sentence is still confirmed as the restart command: the confirmation handles the fragment the speech recognition service for commands produces when the next speech starts about 0.4 s after the command word, in the way `tame-confirming-special-cases` chooses for mishearings (data, not a branch) when that change lands first, else in the confirmation code.
- The scenario passes deterministically: a test reproduces the heard fragment ("Stud, <sentence>") against the confirmation first (Red), then the fix (Green).

Current stage: PoC. Target stage: PoC.
Scope: the confirmation of the restart command in `command-spotting/` and its tests; the E2E scenario stays as it is.
Acceptance bar: the scenario "Restart, a short pause, then speech that starts like another word" and every other `@smoke`/`@regression` voice control scenario pass in `full-check`; a unit or integration test holds the heard fragment; `voice_samples_it` unchanged.
Out of scope: other commands' mishearings; the flake mechanics of the lock classes; the structure of `confirming.rs` (`tame-confirming-special-cases`).

## Capabilities

### Modified Capabilities

- `voice-control`: a restart said right before the next sentence (short pause) is confirmed even when the service hears it as a fragment of the next sentence; no new requirement, the existing restart-after-pause scenario is what must hold.

## Impact

- `command-spotting/src/engine/confirming.rs` or the mishearing table, `command-spotting/src/matcher/**`, `command-spotting/tests/**`; `e2e-test/tests/features/voice_control.feature` only if a scenario needs an example row.
- `refresh-human-docs` (PR #293) waits for this change before `kit land`: its full check is red only on this scenario.

## Open points

- Whether the fix is a mishearing entry ("Stud" as a cut "Restart" when followed by speech within the lead-in window) or a change in how the command confirmation clips the segment at a 0.4 s pause; proposed: first look at the segment the service sent to the recognizer and fix the clipping if the word is cut, else add the mishearing.
- Whether it also fails with the user's own recorded "restart" (`~/.voicepuncher/voice-samples/`); proposed: run `voice_samples_it` first.
- Target version: `0.37.3` (PATCH over `main` 0.37.2); `kit version-check` corrects it when another change bumps first.

## Release

SemVer: PATCH — a bug fix in voice control, no new capability.
Target version: 0.37.3.
