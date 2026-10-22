## Why

The E2E voice control scenario "Clear text is recognized as clear all text" (example row "clear text please", `e2e-test/tests/features/voice_control.feature:166`, step `e2e-test/tests/steps/voice_control.rs:379`) fails without a code change touching voice control. In the `e2e-regression` stage of `just full-check` (light lock class, test ports 27081 and siblings) on branch `chore/fast-test-build` (a test layout change that touches no voice control code) at 16:48 on 2026-10-22 the command spotting service recognized nothing for the synthesized "clear text please" with a pause before and after it: `recognized [], expected only ClearAllText`. The other 40 scenarios of the stage passed, the example row "clear text" of the same outline passed, the light lock was taken after 0.7 s, and cargo printed "Blocking waiting for file lock on package cache" from other sessions' builds at the same time, so the machine was loaded by parallel builds.

A flake is a failure (AGENTS.md § How to verify a change): the check is never rerun unchanged, so the cause of a recognition that returns nothing for a valid command phrase under load must be found and fixed.

## What Changes

- The command spotting service confirms "clear text please" (keyword spotting plus the speech recognition service for commands) under the load of parallel builds, so the scenario passes in every `e2e-regression` run, or the cause of the empty recognition is found and removed (a timeout too short under load, a dropped confirmation, a chunk boundary inside the phrase, or a synthesized sample the models miss).

Current stage: PoC. Target stage: PoC.
Scope: the recognition path of "clear text please" in the command spotting service and its confirmation, and the E2E step that checks it.
Acceptance bar: the scenario passes in `just full-check` runs of three changes on this machine with parallel sessions building beside them (the stage journal shows no failed "Clear text is recognized as clear all text"); the cause is named in the design.
Out of scope: the other flaky scenarios named in the audit of 2026-10-21; the machine lock and the lock classes.

## Capabilities

### Modified Capabilities

- `voice-control`: "clear text please" with a pause before and after it takes effect as the clear all text command, also while the machine runs other sessions' builds.

## Impact

- `command-spotting/src/` (the confirmation of a spotted keyword) and possibly `e2e-test/tests/steps/voice_control.rs`.
- `fast-test-build` waits for this change (`depends_on` in its `orchestration.yaml`), as the flake rule asks.

## Open points

- Whether the empty recognition comes from the service (a confirmation timeout under load) or from the synthesized sample: the proposed first step is to replay the cached synthesized "clear text please" against the service under an artificial load and read the service log.
- Target version: the bump proposed below; the change takes the next PATCH above `main` when it starts.

## Release

SemVer: PATCH — a bug fix of the command spotting service, no new capability.
Target version: 0.37.3.
