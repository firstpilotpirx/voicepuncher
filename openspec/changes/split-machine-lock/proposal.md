## Why

Waiting for the machine-wide test lock took 141 of ~950 check minutes (≈ 15 %) over 140 recorded runs of 14–18 Oct (research of 2026-10-18, report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7); the audit of 2026-10-16 measured lock wait at 35 % of run time. Every model stage of every check takes the same one lock, so a session waits for another session's whole model phase. But the stages load the GPU very differently: the voice control scenarios (127 of the 178 `@smoke` scenarios, and `@resilience`) mostly wait on audio streamed in real time (100 ms frames, 15 s pauses, reactions awaited up to 30 s) and run the small command recognition model, while accuracy (`e2e-quick`, `e2e-bench`), seams and refinement evaluation keep the dictation model and the LLM busy. The user agreed (2026-10-18) to split the machine-wide lock.

## What Changes

- The model stages no longer all take one lock: stages that barely load the GPU (proposed: the voice control scenarios and `*_it` binaries of the command spotting service) take a lock of their own, so one of them can run beside another session's GPU-heavy stage (accuracy, seams, refinement).
- Each run keeps its own services on its own ports, so two locked runs never share a service; test ports are picked per lock.
- The split is backed by measurement: GPU load and stage times of each stage alone and side by side, before and after; a split that slows a stage or makes a test fail is not taken.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`_stages`, `_model-stages`, test ports, E2E recipes), the use of agent-kit `with-machine-lock` (a second lock name), AGENTS.md § How to verify a change and `~/.claude/rules-custom/machine-lock.md` project values.
Acceptance bar: two checks from two worktrees run their light and heavy model stages side by side and both pass; total lock wait per check measured lower than before; no test gets flaky from running beside another stage (repeated runs green).
Out of scope: making stages faster (`speed-up-tests`), reusing passed stages (`reuse-passed-check-stages`), flaky tests (`fix-flaky-tests`).

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `justfile`, `.config/nextest.toml` (if suites move), AGENTS.md § How to verify a change.
- Possibly agent-kit `with-machine-lock` / `machine-lock.md` if named locks or a lock with two slots are missing there (`~/.claude/rules-custom/kit-placement.md`).

## Open points

- Which stages count as light: proposed — decided by measured GPU load, starting with the voice control part of `e2e-smoke` and `e2e-resilience`.
- Whether `e2e-smoke` must split into a voice control stage and the rest to make this work: proposed — yes, if `speed-up-tests` has not already split it.
- Two named locks versus one lock with two slots: proposed — two named locks (light, heavy), so two heavy stages never overlap.
- Ports for two runs at once: proposed — a second fixed set of test ports for the light lock.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
