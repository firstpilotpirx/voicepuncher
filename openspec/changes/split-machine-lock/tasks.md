# Tasks

No Rust code and no spec scenario (`skip_specs: true`): phase split and locks are tested by recipe self-tests, the split itself by measured recipe runs (design.md § Test levels).

## 1. Phases (decisions 1, 5)

- [ ] 1.1 Red: `_test-lock-phases` in the `justfile` expects `_lock-phases` to print the heavy and the light phase of a stage list in order, an unknown model stage as heavy, nothing for a class without stages; run it, it fails (no `_lock-phases`).
- [ ] 1.2 Green: `light_stages` and `_lock-phases`; `_test-lock-phases` passes; the `test` stage runs it; `just test`; commit.

## 2. Class locks and ports (decisions 2–4)

- [ ] 2.1 Red: `_test-class-locks` with scratch lock directories expects two classes to overlap, one class to run one at a time and an exclusive machine-lock run to exclude both; run it, it fails (no `_class-lock`).
- [ ] 2.2 Green: `_class-lock <class> <command…>` (class lock, then the machine lock `--shared`, outer deadline = wait + deadline), light test ports and their variables in the port check; `_test-class-locks` passes; the `test` stage runs it.
- [ ] 2.3 `_stages` runs the heavy and the light phase each under `_class-lock` with its ports, skips a class with every stage reused, one phase when the caller holds the machine lock; stage table and journal show the wait and hold per class (`locks`); `just stage-times` keeps summing; `rerun-failed` runs its stage in its class; `just test`; commit.

## 3. Measurement (decision 6)

- [ ] 3.1 GPU sampler in the scratchpad; each model stage alone (one `full-check` under the exclusive machine lock): GPU load and time per stage, in design.md.
- [ ] 3.2 Two `full-check` runs from two worktrees side by side, twice: both pass, stage times against alone, lock wait per check against one after the other, in design.md; move a stage back to heavy when decision 6 says so; commit.

## 4. Docs

- [ ] 4.1 `AGENTS.md` § How to verify a change and § Several changes at once: the two classes, their tags, ports and locks; `justfile` comments; commit.
- [ ] 4.2 Version: no bump — test tooling only (proposal § Release); manual checks: none.
