## Why

Two tests outside `fix-flaky-output-limit-refinement-scenario` failed in that change's worktree (practical-hofstadter-aa5b44, branch `fix/fix-flaky-output-limit-refinement-scenario`) on 2026-10-22, both under load, and a flake is a failure (AGENTS.md § How to verify a change):

- `just e2e-smoke` at 20:52 (the heavy run started as the load for the change's 20-run `just e2e-regression` series, which ran beside it in the light class): the `e2e-smoke-speech` stage failed in three scenarios of `speech_recognition.feature` — "Silence produces a lone \"you\"" (`Then no text is produced`, the app exited 1), "Recording file longer than the chunk threshold" (100 % of the spoken words missing, printed text empty) and "A domain term named in the prompt is kept" (the printed text lacks "Kubernetes"). The app's log shows the cause: `speech recognition service did not answer in time (request=1 attempt 1, after 18753ms, deadline 18750ms): copy 38084 sent no answer in 18750ms`, the same on copies 38082 and 38083, then `copy 38082 hung: no answer in 37 s` and `file mode failed: speech recognition request to http://127.0.0.1:28081/v1 failed on copies 38084, 38082, 38083`; 15 such warnings in the run. The three `whisper-server` copies of the dictation service (ggml-large-v3-turbo-q5_0, pids 32633 32644 32661) answered nothing for 18 s while the light run's refinement series and the user's own bundle services (six `whisper-server` processes from `dist/`) ran beside them. `e2e-smoke-voice` (161 s) and `e2e-smoke-refinement` (36.8 s) passed in the same run; `e2e-smoke-services` was skipped after the failure. Log: `/private/tmp/claude-501/-Users-pirx-Workspaces-my-voicepuncher--claude-worktrees-practical-hofstadter-aa5b44/64813301-4e25-45da-afbd-3133c6f9d0a5/scratchpad/smoke-1.log`; journal entry `e2e-smoke` at 2026-10-22T20:52:04+02:00.
- `just test` at 21:02 (no locked run beside it; the user's bundle services running): `voicepuncher::it recording_cpal::synthetic::should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine` (`app/tests/it/recording_cpal/synthetic.rs:122`) failed with `left: 944160, right: 960000`: the stand-in recorder lost 15 840 of 960 000 samples of a minute of audio fed in real time with half the cores busy by the test's own CPU hogs, and the test took 73 s (nextest marked it SLOW past 30 s and 60 s) while the `main_thread` window tests ran beside it. The same test failed twice on 2026-10-22 at 11:52 and 11:56 on `fix/realtime-safe-audio-capture`, the branch that introduced it. Log: `/var/folders/h8/wy4hskxs1497zt08m6dt9xrh0000gn/T/run-visible/20261022-210219-just-test.log`.

Neither test is in the scope of `fix-flaky-output-limit-refinement-scenario`, whose branch waits for this fix (`depends_on`). `fix-flaky-restart-cue-under-metal-load` (the command service's copies stalled by the Metal compiler in `e2e-cues`) and `fix-placed-commands-real-service-under-load` (the command spotting service's real-service tests) cover other tests of the same family; the dictation service's copies stalling for 18 s under load in `e2e-smoke-speech` is a new site, and so is the recording test.

## What Changes

- The `e2e-smoke-speech` scenarios give the same result beside a light-class run and the user's own services: the dictation service's copies answer within the app's deadline under that load, or the stage proves the copies answer before its first scenario and names the stalled copy and the load (the busiest processes, the lock holders) when they do not — without widening the app's deadline.
- `should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine` keeps every sample on a busy machine as its name says, or measures what it can prove under `just test`'s own parallel load (the `main_thread` tests beside it) with a bound it meets 20 times in a row; the lost 15 840 samples are explained first (the stand-in feeder, the ring buffer, the test's hogs).

Current stage: PoC. Target stage: PoC.
Scope: `e2e-test/tests/features/speech_recognition.feature` and its steps, the smoke stage's service start in the `justfile`; `app/tests/it/recording_cpal/synthetic.rs` and the stand-in recorder it drives.
Acceptance bar: `just e2e-smoke` passes 3 times in a row with `just e2e-regression` from another worktree running beside it; the recording test passes 20 times in a row inside `just test`; `just test` and `just check` green.
Out of scope: the app's speech recognition deadlines and retries; the command service's stalls (`fix-flaky-restart-cue-under-metal-load`); the real-service tests of the command spotting service (`fix-placed-commands-real-service-under-load`).

## Capabilities

### Modified Capabilities

- `speech-recognition`: the file-mode scenarios hold under a light-class run beside the stage; the requirement that a recording loses no audio under load (it lives in this capability) states how long the recording holds back audio and what a failed check of it reports.

## Impact

- `e2e-test/tests/features/speech_recognition.feature`, `e2e-test/tests/steps/speech_recognition.rs` and `speech_recognition/prompt.rs`, `justfile` (smoke stage service start), `app/tests/it/recording_cpal/synthetic.rs`, `app/src/adapters/secondary/recording/cpal/` (only when the lost samples are the adapter's).

## Open points

- Whether the 18 s stall of the dictation copies is Metal shader compilation on a fresh copy (as in `fix-flaky-restart-cue-under-metal-load`) or GPU contention with the user's six bundle `whisper-server` processes: proposed to measure both before choosing the fix.
- Whether the lost samples are the test's stand-in feeder falling behind real time under the hogs or the adapter's ring buffer: proposed to instrument the test first; a PATCH release only when the adapter is at fault.

## Release

SemVer: no release — test and recipe changes only; PATCH (0.37.4) when the recording adapter loses samples.
Target version: 0.37.3.
