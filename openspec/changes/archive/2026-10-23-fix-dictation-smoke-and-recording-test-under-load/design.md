# Design

## Context

See proposal.md § Why. Two facts shape the approach:

- **The smoke stage floods the copies.** The harness runs 6 scenarios at once, each file-mode scenario an app process with its own copy pool, so 6 processes lease the 3 copies unseen by each other, and the smoke set holds two 60-second silence files in 20-second chunks. At the failure every short request (deadline 5 s + 0.69 s per audio second) queued behind a chunk on the same copy, and the chunks passed 18.75 s once the light run shared the GPU. No decoder fallback (`temperature_inc` is 0); the copies were warm.
- **The recording test cannot say what it lost.** It asserts the length first, so the ring's dropped count is never shown. Alone and beside the `main_thread` tests it passed here. The ring holds 2 s; losing 0.99 s means the drain thread was away about 3 s. The feeder's 10 ms sleeps overshoot under load (73–84 s per minute).

## Goals / Non-Goals

**Goals:** the smoke-speech stage holds beside a light-class run; a stalled copy is reported with the load; the recording test names its loss; the adapter survives a drain held up for seconds.

**Non-Goals:** the app's deadlines, retries and copy pool; a copy warm-up (the restart-cue change); the `@seams` scenarios' concurrency (own stage, never failed).

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Queueing on the copies | every `@smoke` scenario that transcribes a recording file gets `@serial` (three features); the harness runs `@serial` alone | deterministic; the stage grows (measured in apply, nextest limit by the 2× rule) | concurrency 3 (collisions remain); a cross-process lease (app); a warm-up (copies were warm) |
| Naming the load | the file-mode When steps store `machine_load::report()` in the world when the app's log shows a request not answered in time; the Then steps that print the log print it | only the When step sees the moment of the failure; the helper exists | a probe step before the scenarios (blind to the load at the failure) |
| What the recording test proves first | the ring's dropped count, as dropped audio in ms with the minute's wall time, before length and order | names the cause next time (the proposal's open point) | a loss bound (the spec says every sample) |
| A drain held up for seconds | `RING_SECONDS` 2 → 10 (1.9 MB at 48 kHz); the busy-machine test takes its ring from the constant | a 3 s stall loses nothing; the command spotting capture shares the margin | a higher drain-thread QoS (an `unsafe` libc call, useless when swapping); a bigger ring in the test only |
| The test's length | a real-time minute; a nextest override gives this test 3 min (2 × 84 s) | real time keeps the ring's timing honest | faster feeding (more load-sensitive); a higher default limit for every test |
| Release | PATCH 0.37.4 | the ring size is the adapter's | — |

## Dependencies

No running change reworks the capture ring, the harness or these scenarios (the peers share only `AGENTS.md`, `justfile` lines and the `it` binary's module list). The acceptance series of task 3.1 (`just e2e-smoke` 3 times beside `just e2e-regression`) failed on its first run in the `e2e-smoke-services` stage, on the local-services scenario "Requests that arrive at once are shared by the copies" (2 of 9 requests past the 5.4 s deadline with the regression run beside it), a test this change does not touch: its fix is filed as `fix-copies-shared-scenario-under-load`, and this change runs the series again once that fix is on `main` (`kit sync-change` refuses a `depends_on` entry for a change filed after this one started, so the wait is not declared in `orchestration.yaml`; the orchestrator was told).

## Test levels

| Spec scenario | Level |
|---|---|
| Recording files are transcribed beside another check | measurement: `just e2e-smoke` 3 times beside `just e2e-regression` from another worktree, plus the `@serial` tags |
| A copy that does not answer in time is reported with the load | unit (plain Rust, `e2e-test/src`): the copies read from the app's log, the report attached; the E2E Then steps print it |
| A long dictation on a busy machine is complete | adapter integration (`recording_cpal/synthetic.rs`): dropped audio and wall time in the failure; 20 runs inside `just test` |
| Audio delivered while the recording is held up is kept | adapter integration: the drain held up 5 s on the adapter's ring size |
| Audio dropped is reported once | adapter integration (exists) |

## Risks / Trade-offs

- [The smoke-speech stage passes half its 2 min limit] → set the limit to twice its longest run in apply.
- [The series loses samples with no drop counted] → the cause is outside the ring; a `scope` call.
- [A silence chunk alone passes 18.75 s under a light run] → the 3-run series shows it; the deadline stays out of scope, the finding is filed.

Results (2026-10-22): `e2e-smoke-speech` with the serial file transcriptions: the harness test 35.6 s alone and 56.7 s beside `just e2e-regression` (the stage 61 s with its service start), so the limit stays 2 min (twice 56.7 s is under it); the busy-machine recording test 20 of 20 runs inside `just test` passed, longest 85.2 s (limit 3 min); the smoke series beside `just e2e-regression`: run 1 failed on the local-services copies scenario (filed as `fix-copies-shared-scenario-under-load`, § Dependencies), the series runs again once that fix is on `main`.
