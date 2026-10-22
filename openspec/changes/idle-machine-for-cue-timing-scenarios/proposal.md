## Why

The E2E scenario "Restart restart followed by the new dictation at once gives its cue at once" (`e2e-test/tests/features/voice_control.feature:411`, step `e2e-test/tests/steps/voice_control/session/start_cue.rs:303`, stage `e2e-seams` of `just full-check`) fails when another worktree's unlocked stages run beside it. On 2026-10-22 13:27 the full-check of `fix/voice-commands-under-load` (commit f6c22507, log `/var/folders/h8/wy4hskxs1497zt08m6dt9xrh0000gn/T/run-visible/20261022-131004-sh--c.log`) lost it with "start cue 1810 ms (words end at 10.62 s)" while the full-check of `fix/realtime-safe-audio-capture` (worktree `interesting-diffie-ec2773`, log `20261022-131313-sh--c.log`) ran its lint, test, metrics, release build, bundle and dmg stages — cargo builds, which take no machine lock — on the same machine; its light phase had not started yet.

The command spotting service log (`~/.voicepuncher/logs/voicepuncher-command-spotting-28082.log`, 11:27:21–26Z) shows the whisper-small copies on ports 38086 and 38088 taking over 1 s for 1.6 s segments (two confirmations cut short at 1000 ms) and the "restart restart" confirmation answered in 1406 ms, against 145–270 ms when idle: the command was heard, the cue came late. The same scenario failed once before, 2026-10-20 16:30 on `feat/app-bundle-and-dmg`.

The voice-control spec requires the 0.7 s cue "with the services otherwise idle" (`openspec/specs/voice-control/spec.md:697`), a condition the stage runner cannot grant while the unlocked build stages of another checkout load the CPU. Related: the design of `split-machine-lock` (a light phase runs beside a heavy one) and the pace of `voice-commands-under-load` (a slow confirmation is now heard late rather than cut short).

## What Changes

- The cue-timing scenarios of the voice-control E2E suites get the idle machine the spec assumes, so the 0.7 s bar is measured under the condition it states; the bar itself stays.
- The check that ran beside the scenario (other checkouts' builds, the `[machine-lock]` holder) is visible in the failure or the stage table, so a cue-timing failure names its load.

Current stage: PoC. Target stage: PoC.
Scope: the stage runner (`justfile` lock classes and build stages), the cue-timing E2E scenarios and their steps, and the stage journal's record of what ran beside a model stage.
Acceptance bar: `just full-check` of a branch passes the cue-timing scenarios while another worktree's `just check` runs its lint, test and build stages, in a series of three runs; the 0.7 s bar is unchanged in the spec and the feature files.
Out of scope: the confirmation pace of the command spotting service; the dictation speech recognition path.

## Capabilities

### Modified Capabilities

- `voice-control`: the cue-timing scenarios state and get the idle services their requirement assumes.

## Impact

- `justfile` (lock classes of `e2e-seams` and the other suites with cue-timing scenarios, or builds yielding to a running model stage), `e2e-test/tests/features/voice_control.feature`, `e2e-test/tests/steps/voice_control/session/start_cue.rs`, the stage journal (`~/.cache/voicepuncher/stage-times.jsonl`).

## Open points

- How the scenarios get an idle machine: the seams stage's cue scenarios run under the exclusive machine lock (proposed), or the builds of other checkouts yield to a running model stage, or the timing bar is measured against the services' confirmation time with the load logged.
- Whether the same treatment covers the light-phase cue scenarios (`e2e-smoke-voice`, `e2e-regression`), which run beside heavy phases by design.

## Release

SemVer: no release — the product does not change; only the checks and the test runner.
Target version: unchanged.
