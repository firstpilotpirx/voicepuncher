# Design

## Context

See proposal.md § Why. The scenario runs the app in file mode once against a stub of the text refinement service (`e2e-test/tests/steps/refinement_service_stub.rs`) that answers one cut-off completion. The stub serves on one thread, records chat bodies only, silently drops a connection it cannot parse and does not record the startup model-list request. The step now checks the app's exit status and pasted text first, so the blind spot left is the stub's side: a connection dropped, or none at all.

## Goals / Non-Goals

**Goals:** a failure names its cause on the first run; a reproduction under load that stays as a guard.

**Non-Goals:** no new timeout, retry or budget anywhere; no change to the other refinement scenarios or to the smoke stage's length.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| What the stub records | Every connection as an event: `models`, `chat` (with body) or `dropped` with the reason | Zero chat requests then reads as "no connection" or "dropped because …" | Drops only: an app that exits at the startup check stays invisible |
| How the step reports | The missing-request failure lists the stub's event log and the app's stderr | Both sides in one message, no second run | Printing during the run: lost in captured output |
| Where the 20-run series lives | A `@regression` scenario in `text_refinement.feature`: one step runs the app 20 times against one stub, asserting one request and the raw transcript each time | `e2e-regression` is a light stage: in `full-check` it runs beside other runs' heavy phases, the load the flake appeared under, and `check` stays as long as it is; the shared speech recognition service keeps a run at a few seconds | `@smoke`: a minute more on every `check`; `@resilience`: its scenarios start services of their own, 20 model loads |
| How the acceptance series runs in apply | `just e2e-regression` beside a heavy E2E run (`just e2e-smoke` from this worktree after its build, or another session's check) | The spec scenario's load; the stage journal records it | `kit verify --series 20`: 20 full checks for one 60 s scenario |
| What gets fixed | Whatever the series names: stub, step or the app's file-mode path; when 20 runs pass, the change ends with the diagnostics and the guard | A fix without a reproduction is a guess | Speculative stub hardening |

The refinement paths in `stage_inputs` (`justfile`) gain `e2e-regression`; the `e2e-regression` limit (`.config/nextest.toml`) is remeasured and moves to twice the new longest run when the stage passes half of it.

## Test levels

| Spec scenario | Level |
|---|---|
| Service stops between recordings; Service hits its output limit | E2E `.feature`, `@smoke` (unchanged) |
| Service hits its output limit while the machine is under load | E2E `.feature`, `@regression` |
| Missing refinement request is told apart from a lost transcription | Unit: the stub records a dropped connection with its reason and a model-list request; the step's failure message carries the event log and stderr |

## First series

Beside `just e2e-smoke`, all 20 runs sent their request and pasted the raw transcript: the lost transcription did not reproduce. The series named the stub: the copy pool's liveness probes (a connection closed without a byte, about two per run) counted as drops; now they are recorded as probes. The heavy run beside it failed in `e2e-smoke-speech`, untouched here and filed (§ Filed beside it).

## Filed beside it

`fix-dictation-smoke-and-recording-test-under-load`: `e2e-smoke-speech` (beside the first series) and the `recording_cpal` busy-machine test (in `just test`) failed under load in this worktree, outside this change's scope. `kit sync-change` refuses a dependency on a change that has not started, so this change does not wait for it; its own checks must pass on their own.

## Risks / Trade-offs

- [20 passes, flake unexplained] → the diagnostics stay; the next failure names its side and gets a fix change.
- [`e2e-regression` grows by about a minute] → `full-check` only; limit remeasured here.
- [A cause in the app's file-mode path] → in scope by the proposal; version moves to PATCH, `kit version-check` rerun.
