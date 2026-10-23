## Why

Checks fail far more often than the code changes justify. In the stage journal (`~/.cache/voicepuncher/stage-times.jsonl`) the voice control smoke stage `e2e-smoke-voice` passed 8 of 8 runs on 2026-10-20 and 2026-10-21, then failed 8 of 30 on 2026-10-22 and 11 of 61 on 2026-10-23; 48 of 66 `full-check` runs failed since 2026-10-22. The failures began at 2026-10-22 01:12, a few hours after `split-machine-lock` landed (#259, 2026-10-21 18:24) and when the orchestrator started many changes in parallel. That change split the machine lock into a light class (`e2e-smoke-voice`, `e2e-regression`, `e2e-resilience`) and a heavy class, and lets one run's light phase run beside another run's heavy phase, so the timing-sensitive voice tests share the GPU with another check's models.

The failing tests are almost all timing-sensitive voice tests that have nothing to do with the branch under check:

- `voice_forms::should_carry_out_every_recorded_way_of_saying_the_commands` (6 times);
- `placed_commands::should_hear_a_chain_of_commands`, `placed_commands::should_hear_a_stop_right_after_the_last_dictated_word`;
- the scenarios "Restart, a short pause, then the new dictation", "A chain of commands takes effect in order", "Requests wait their turn when every copy is busy".

Each such failure makes a change session freeze the test and rerun a 20–50 min `full-check`, so one session collected six red `kit verify` runs in a row (screenshot below), and frozen tests pile up.

![Agent panel of a change session: six failed kit verify runs in a row](assets/be1dde18b549.webp)

The user wants every test and check stage that uses the GPU or a model engine to run strictly one at a time on the machine, in exclusive mode.

## What Changes

`run-e2e-alone-on-the-machine` (#307, merged 2026-10-23 after this change was filed) already removed the light/heavy split: every model stage of a run runs in one phase under the machine lock held exclusively, and builds and tests of other runs wait while it runs. This change does what is left of the user's request:

- Every model stage that uses the GPU or a model engine stays under that one exclusive lock; no second mechanism is added.
- The stage journal records, for each lock a run held, when the hold began, and `just stage-times` reports whether two runs' model-stage holds overlapped (none, or each overlapping pair), so an overlap can be ruled out from the journal.
- Builds, `fmt`, `lint`, `metrics` and `just release` keep running in every session at once (unchanged).
- Tests frozen as load flakes in the sessions of 2026-10-22..23 (among them `voice_forms` frozen whole) are unfrozen when they pass under the exclusive lock; one that fails with the machine to itself is not a load flake and stays frozen, named in `tasks.md`.

Current stage: PoC. Target stage: PoC.
Scope: the stage journal's lock records and `just stage-times`, `.config/frozen-tests.toml`, the AGENTS.md text on the journal.
Acceptance bar: `just stage-times` reports no overlapping model-stage holds; every unfrozen test passes in a run of its stage; `e2e-smoke-voice` passes in every `full-check` over at least the first 20 runs after landing (measured after the merge with `just stage-times`); builds and unit tests still start without waiting for the lock when no model stage runs.
Out of scope: the app's own runtime behavior; speeding the model stages up; the number of changes the orchestrator runs at once; fixing a test that fails with the machine to itself.

## Capabilities

### Modified Capabilities

- `change-workflow`: the stage journal shows when each lock hold began, and the stage summary reports overlapping holds.

## Impact

- `justfile` (`_stage-journal-append`, `stage-times`, a new self-test), the stage journal.
- `AGENTS.md` § How to verify a change (Lock classes, the stage journal).
- Tests frozen as load flakes in `app/`, `command-spotting/`, `e2e-test/`.

## Open points

- Settled by `run-e2e-alone-on-the-machine`: the quiet lock stays; a run holds the lock once for all its model stages; the longer total check time is accepted.
- How to find every test frozen as a load flake: every entry of `.config/frozen-tests.toml` on `main` is dated 2026-10-23 and marked a load flake; all of them are candidates.

## Release

SemVer: no release — only the test tooling changes, the app's behavior does not.
Target version: unchanged.
