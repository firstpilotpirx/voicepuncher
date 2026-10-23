## Why

The `full-check` of `refresh-human-docs` (worktree `sleepy-kapitsa-e5d9b5`, commit `f0e051d1`, 2026-10-22 16:28, log `run-visible/20261022-162801-sh--c.log`) failed in the stage `e2e-smoke-voice` (light lock class) on the `@smoke @serial` scenario "Restart, a short pause, then speech that starts like another word" (`e2e-test/tests/features/voice_control.feature`): "no dictation started; events: [Listening, RecordingStarted]" — the command spotting service reported no restart. Beside the run: `just test` of two other worktrees.

The first reading of that failure was a mishearing: the line "Stud, coffee machine on the third floor is broken." on the harness's stderr. Investigation in this change shows it is not:

- The stderr lines are the pasted dictations of the part's scenarios in order; the failing scenario pasted nothing. "Stud, …" is the paste of the first scenario of the part ("Start, start" and a sentence in one breath), which passed.
- The same audio as the scenario ("Weather is sunny in Berlin today", then the voice Eddy's "Restart", 0.4 s of silence and "Coffee machine on the third floor is broken"), fed in real time to the real keyword spotter and confirmed by a real `whisper-server-commands` (small): the segment is heard "Restart!" and "restart dictation" is reported 20 of 20 times on an idle machine, and 10 of 10 times under twelve CPU-bound processes, with the code of `main` and with the code before `voice-commands-under-load`.
- The failing commit `f0e051d1` did not contain `voice-commands-under-load` (PR #288, merged 16:16, 12 minutes before the run started on a branch cut earlier). That change fixed exactly this symptom under exactly this load: commands lost while other sessions' `just test` runs load the machine, because each confirmation was cut short at the 1 s decoding budget of an idle machine ("confirmation cut short after 1000ms …: still decoding, nothing heard"), and copies were dropped after one slow probe.
- In the stage journal the scenario failed once in 82 voice smoke runs: that run.

The command spotting service's log of the run (`voicepuncher-command-spotting-27082.log`) was overwritten by a later run, so the cut-short line cannot be shown; the restart after a short pause is the one placement the under-load tests of #288 do not cover.

## What Changes

- No change to the confirmation: no mishearing entry for "Stud" (none was heard), no change to the clip.
- A regression test holds the case under load: "Restart" said in a dictation, a 0.4 s pause and a sentence, confirmed by a copy of the speech recognition service for commands that decodes every confirmation for 1.2 s (the loaded copy of #288's tests), is reported as "restart dictation". It fails with the decoding budget before #288 and passes on `main`.
- The voice control spec gains that scenario under its requirement "A command confirmed slowly under load is still carried out".

Current stage: PoC. Target stage: PoC.
Scope: the under-load tests of `command-spotting/tests/it/placed_commands/` and the voice control spec; the E2E scenario stays as it is.
Acceptance bar: the new test passes with `main`'s decoding budget and fails with a fixed 1 s budget; the scenario "Restart, a short pause, then speech that starts like another word" and every other `@smoke`/`@regression` voice control scenario pass in `full-check`; `voice_samples_it` unchanged.
Out of scope: other commands' mishearings; the flake mechanics of the lock classes; the structure of `confirming.rs` (`tame-confirming-special-cases`); keeping the services' logs of a failed E2E run.

## Capabilities

### Modified Capabilities

- `voice-control`: the requirement "A command confirmed slowly under load is still carried out" gains the scenario of a restart before a short pause and more speech.

## Impact

- `command-spotting/tests/it/placed_commands/under_load/mod.rs` (new test and its audio).
- `openspec/specs/voice-control/spec.md` (through the delta spec).
- No production code; the version stays (no behavior change, no release).
- `refresh-human-docs` (PR #293) waits for this change before `kit land`.

## Release

SemVer: none — no behavior change; the version stays and no release is cut.
