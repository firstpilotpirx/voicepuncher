## Why

The exclusive `e2e-cues` stage of `just full-check` fails now and then on one scenario: "Restart said on its own gives its cue at once" (`e2e-test/tests/features/voice_control.feature:414`, step "the start cue of the new dictation comes within 0.7 s of the end of \"restart\"", `e2e-test/tests/steps/voice_control/session/start_cue.rs:307`) with `"restart" alone: no start cue (words end at 30.53 s of the stream)`. The stage journal shows it three times: 2026-10-19 (`fix/fix-flaky-tests`), 2026-10-20 (`fix/match-late-confirmation-to-its-state`) and 2026-10-22 19:57 on `chore/slim-pipeline-module` (commit 4f2fe596, log `/var/folders/h8/wy4hskxs1497zt08m6dt9xrh0000gn/T/run-visible/20261022-194323-sh--c.log`); every other `e2e-cues` run of 2026-10-22 (five branches, 14:29–18:29) passed. The scenario is not load-sensitive by accident: `e2e-cues` already runs with the machine lock held exclusively (archived `idle-machine-for-cue-timing-scenarios`), and at the last failure the lock holder was the stage's own `_model-stages e2e-cues`; the busiest processes were three `MTLCompilerService` processes at 264–310 % CPU (Metal shader compilation) beside `whisper-server` at 18 %.

The command spotting service's log of that run (`~/.voicepuncher/logs/voicepuncher-command-spotting-28082.log`, 17:58:50–17:58:56 UTC) shows what happened: the keyword spotter fired `trigger "restart trigger" at 29.69–30.58s`, then both confirmations were cut short: `confirmation cut short after 1000ms on copy http://127.0.0.1:38088/v1: still decoding, nothing heard` (rehear) and the same on copy 38087 (segment), both answering empty after 1.2–1.3 s, while the stream's audio lag rose to 1.0–1.3 s ("audio waited for processing") for the next 4 s. The copies answered the dictation checks before it in 120–370 ms. So the command was heard, but the speech recognition service for commands could not confirm it within its 1 s budget because its copies were stalled by the Metal compiler, and the app gave no start cue.

The `justfile` warms nothing beyond readiness probes: a freshly started `whisper-server` copy compiles its Metal shaders on its first real decode, and the exclusive stage starts its own services right before timing the cue, so the first confirmations of the stage can land on a copy still compiling.

## What Changes

- A copy of a speech recognition service counts ready only after it transcribed a short utterance: the service scripts (`just start-model-servers`, so every E2E stage and `e2e-cues` start on warm copies) and the app's own service start (so the user's first dictation and first command never wait for a copy's first decode) both warm every copy they start; the first transcription's time is reported with the copy's readiness.
- The failure of a cut-short confirmation under the exclusive stage names the cause (copies stalled, the `MTLCompilerService` load) instead of only "no start cue", so the next such failure needs no log reading.
- No change to the 0.7 s bound, the 1 s confirmation budget or any other bar of the specs.

Current stage: PoC. Target stage: PoC.
Scope: the `e2e-cues` stage of `check`/`full-check`, the warm-up of the whisper copies it starts (justfile recipes or the E2E harness), the failure message of the cue scenarios.
Acceptance bar: `just e2e-cues` passes 10 runs in a row from freshly started services (`VOICEPUNCHER_REUSE_STAGES=0`), the "restart" scenario included, with no widened bound or budget; the design names where the warm-up lives and proves a cold copy no longer answers the first timed confirmation.
Out of scope: the confirmation budget of the command spotting service, the other flaky E2E scenarios (filed separately), the load of other sessions' checks (the stage is already exclusive).

## Capabilities

### Modified Capabilities

- `voice-control`: the start cue requirement keeps its bound; it is timed on warm copies, and a missing cue names the confirmations cut short.
- `local-services`: a copy of a speech recognition service is ready once it transcribed speech, in the service scripts and in the app.

## Impact

- `justfile`: `start-model-servers` (the readiness of a whisper copy: a warm-up transcription), a committed warm-up utterance `app/assets/warm-up.wav`.
- `app/src/core/application/local_services/` (the supervision counts a started copy ready after its warm-up), `app/src/adapters/secondary/localservices/process/probe.rs` (the warm-up request).
- `e2e-test/tests/features/local_services.feature` (the scripts report a copy ready after its first transcription).
- `e2e-test/tests/steps/voice_control/session/start_cue.rs`, `e2e-test/src/` (a missing cue's failure names the confirmations the command spotting service cut short and the machine's load).

## Open points

Settled at preparation (`orchestration.yaml`): the warm-up lives in the service start, scripts and app alike; its depth comes from a measurement of the Metal shader cache (design); the 2026-10-19 and 2026-10-20 failures count as the same cause, verified by the 10-run series.

## Release

SemVer: PATCH — the app counts a speech recognition copy it started as ready only after its first transcription, so the first dictation or command no longer waits for a cold copy.
Target version: 0.37.5.
