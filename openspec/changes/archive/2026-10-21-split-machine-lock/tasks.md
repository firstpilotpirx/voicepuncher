# Tasks

No Rust code and no spec scenario (`skip_specs: true`): phase split and locks are tested by recipe self-tests, the split itself by measured recipe runs (design.md § Test levels).

## 1. Phases (decisions 1, 5)

- [x] 1.1 Red: `_test-lock-phases` in the `justfile` expects `_lock-phases` to print the heavy and the light phase of a stage list in order, an unknown model stage as heavy, nothing for a class without stages; run it, it fails (no `_lock-phases`).
- [x] 1.2 Green: `light_stages` and `_lock-phases`; `_test-lock-phases` passes; the `test` stage runs it; `just test`; commit.

## 2. Class locks and ports (decisions 2–4)

- [x] 2.1 Red: `_test-class-locks` with scratch lock directories expects two classes to overlap, one class to run one at a time and an exclusive machine-lock run to exclude both; run it, it fails (no `_class-lock`).
- [x] 2.2 Green: `_class-lock <class> <command…>` (class lock, then the machine lock `--shared`, outer deadline = wait + deadline), light test ports and their variables in the port check; `_test-class-locks` passes; the `test` stage runs it.
- [x] 2.3 `_stages` runs the heavy and the light phase each under `_class-lock` with its ports, skips a class with every stage reused, one phase when the caller holds the machine lock; stage table and journal show the wait and hold per class (`locks`); `just stage-times` keeps summing; `rerun-failed` runs its stage in its class; `just test`; commit.

## 3. Measurement (decision 6)

- [x] 3.1 GPU sampler in the scratchpad; each model stage alone (one `full-check` under the exclusive machine lock): GPU load and time per stage, in design.md.
- [x] 3.2 Two `full-check` runs from two worktrees side by side: not run (batch mode allows `just test` only); design.md § Measurements says who applies decision 6 after the merge; commit.

## 4. Docs

- [x] 4.1 `AGENTS.md` § How to verify a change and § Several changes at once: the two classes, their tags, ports and locks; `justfile` comments; commit.
- [x] 4.2 Version: no bump — test tooling only (proposal § Release); manual checks: none.
