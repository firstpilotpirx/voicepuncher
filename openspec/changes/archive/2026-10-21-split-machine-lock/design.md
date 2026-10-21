# Design

## Context

`_stages` runs the unlocked stages, then every model stage that is not reused in one phase, `_model-stages`, under the exclusive machine lock (agent-kit `with-machine-lock`, one lock file per lock directory, modes exclusive and `--shared`). The phase starts the four services on the test ports 28081/28085/28080/28082 once and stops them at its end. reuse-passed-check-stages (stacked) splits the smoke set into `e2e-smoke-speech`, `-refinement`, `-voice` and `-services`.

## Goals / Non-Goals

**Goals:** a check's voice control stages run beside another check's dictation and refinement stages; no test gets flaky from it; lock wait per check drops.

**Non-Goals:** no agent-kit change; standalone recipes outside `_stages` (`fmt`, `lint`, `build`, `test-real-keyboard`) keep the exclusive lock; no stage gets faster on its own.

## Decisions

1. **Two classes of model stage.** `light`: the voice control stages `e2e-smoke-voice`, `e2e-regression`, `e2e-resilience` — audio streamed in real time, the small command model. `heavy`: every other model stage (`e2e-smoke-speech`, `-refinement`, `-services`, `e2e-seams`, `e2e-quick`, `e2e-bench`, `refine-eval`). A `justfile` variable `light_stages` names the light ones; a stage it does not name is heavy, so a new stage never runs beside another by mistake. A light stage stays light only when measurement backs it (decision 6).
2. **One lock per class, plus the machine lock shared.** Each class phase takes the lock directory of its class (`<machine lock dir>-voicepuncher-<class>`) exclusively, then the machine lock with `--shared`. Two runs of one class never overlap; a light and a heavy phase do; any exclusive run on the machine (another project, a standalone recipe) still has the machine alone, because the kit lets an exclusive waiter go before new shared runs. Class lock first, so a run never holds the shared machine lock while it queues for its class. Rejected: one lock with two slots (two heavy phases could overlap); a kit change for named locks (a lock directory already is one).
3. **The outer deadline covers the inner wait.** The class lock's hold includes the wait for the machine lock, so its `MACHINE_LOCK_TIMEOUT` is the inner wait limit plus the inner deadline (`MACHINE_LOCK_WAIT` + `MACHINE_LOCK_TIMEOUT`, default 7200 s); the machine lock keeps the real deadline on the stages.
4. **Ports per class.** Heavy keeps 28081/28085/28080/28082 (copies 38082–38089); light gets 27081/27085/27080/27082 (copies 37082–37089), set by `VOICEPUNCHER_TEST_LIGHT_*_PORT`. Each phase starts and stops its own four services, so two phases never share one; a held port names the variable of its class.
5. **Phases in a run.** `_stages` splits the model stages to run into a heavy and a light phase, keeping their order within each, and runs the phases one after another in the order of their first stage; a class with every stage reused takes no lock. When the caller already holds the machine lock, all model stages run in one phase on the heavy ports, as before. The stage table shows the wait and hold per class; the journal keeps `lock_wait`/`lock_held` as the sums and adds `locks` with each class's wait and hold.
6. **Measured, not assumed.** GPU utilization (IOAccelerator `Device Utilization %`, sampled each second through `ioreg`) and stage times: each model stage alone, then two `full-check` runs from two worktrees side by side, twice. A light stage moves back to heavy when side by side it fails, or its time or the time of the heavy stage beside it grows by more than 20 % over alone. The results go into this file and the PR.

## Test levels

No spec scenario (`skip_specs: true`): test tooling only.

| Acceptance item | Level |
|---|---|
| model stages split into the heavy and light phase in order; unknown stage is heavy; reused stages leave a phase out | recipe self-test `_test-lock-phases`, run by the `test` stage |
| two class locks overlap, two runs of one class do not, an exclusive machine-lock run excludes both | recipe self-test with scratch lock directories and `sleep` commands |
| GPU load and time of each model stage alone | recipe run with the `ioreg` sampler |
| two `full-check` runs side by side pass, twice; lock wait lower than one after the other | recipe runs (stage journal) |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Risks / Trade-offs

- [Voice control timing tests slow down beside a heavy stage] → decision 6 moves such a stage back to heavy.
- [Two service sets need memory: two dictation models with three copies each, two LLMs] → 64 GB machine; measured in decision 6.
- [A run holding the class lock waits for an exclusive machine-lock run] → the other class still runs; the class waiter adds nothing to the machine queue.

## Measurements

Each model stage alone under the exclusive machine lock, 2026-10-21, IOAccelerator `Device Utilization %` each second over the stage (service start included):

| Stage | Class | Time | GPU mean | GPU p90 | GPU memory max |
|---|---|---|---|---|---|
| e2e-smoke-speech | heavy | 61 s | 36 % | 100 % | 11.2 GB |
| e2e-smoke-refinement | heavy | 26 s | 40 % | 100 % | 10.6 GB |
| e2e-smoke-voice | light | 315 s | 46 % | 94 % | 10.0 GB |
| e2e-smoke-services | heavy | 99 s | 32 % | 100 % | 13.5 GB |
| e2e-regression | light | 32 s | 57 % | 97 % | 9.7 GB |
| e2e-seams | heavy | 454 s | 53 % | 99 % | 11.1 GB |
| e2e-resilience | light | 127 s | 35 % | 95 % | 4.2 GB |
| e2e-quick | heavy | 62 s | 69 % | 99 % | 10.6 GB |
| refine-eval | heavy | 39 s | 63 % | 95 % | 10.6 GB |

Voice control is not GPU-light: its mean load is in the range of the heavy stages, none of which keeps the GPU busy (means 32–69 %). A light run took its lock and ports (27xxx) and passed (`e2e-regression`, 73 scenarios). The side-by-side runs of decision 6 did not run in this change: the orchestrator's batch mode allows `just test` only; the batch's full check runs the split, and the first two side-by-side checks after the merge (stage journal, `locks`) apply decision 6's rule.

## Dependencies

Stacks on reuse-passed-check-stages (declared): it rewrites `_stages` and splits the smoke set this change classifies.
