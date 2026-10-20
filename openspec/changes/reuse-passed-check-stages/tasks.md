# Tasks

No Rust code and no spec scenario (`skip_specs: true`): the key is tested by a recipe self-test, reuse by recipe runs with `VOICEPUNCHER_STAGE_PASSES` and `VOICEPUNCHER_STAGE_JOURNAL` at scratch paths (design.md § Test levels). Scratch edits are never committed.

## 1. Stage keys (decisions 1–2)

- [ ] 1.1 Red: `_test-stage-keys` in the `justfile` builds a scratch git repository and expects `_stage-keys` to print one key per stage that changes with an input file, an untracked file, a deleted file and the stage name, and not with a `none` file, another model stage's file or a version-only Cargo edit for a model stage; run it, it fails (no `_stage-keys`).
- [ ] 1.2 Green: `stage_inputs` table and `_stage-keys` (file hashes, toolchain, runtime inputs, env filter); `_test-stage-keys` passes.
- [ ] 1.3 The `test` stage runs `_test-stage-keys`; `just test`; commit.

## 2. Smoke split (decision 3)

- [ ] 2.1 nextest profiles `e2e-smoke-voice`, `-speech`, `-refinement`, `-services`, `-rest` in `.config/nextest.toml`; recipes `_e2e-smoke-<part>`; `e2e-smoke`, `check` and `full-check` list the five stages; `stage_inputs` names each part's files.
- [ ] 2.2 `just e2e-smoke` runs every `@smoke` scenario and smoke binary once (count before and after); each part's limit set from its measured time (AGENTS.md § Test limits); commit.

## 3. Reuse in `_stages` (decisions 4–5)

- [ ] 3.1 Red: with scratch passes and journal, `just _stages fmt` twice runs `fmt` twice.
- [ ] 3.2 Green: `_stages` looks up each key, shows `↺ reused` with its source, runs the rest, keeps `build`/`build-e2e` when a model stage runs, skips the lock when no model stage runs, writes a pass record after a pass whose key held; `VOICEPUNCHER_REUSE_STAGES=0` runs all. Check: the second `just _stages fmt` reuses `fmt`; an untracked `.rs` file reruns it; an untracked file under `docs/` does not.
- [ ] 3.3 Journal: a reused stage is `reused` with `from`; `just stage-times` counts reused stages apart and leaves them out of medians and totals; commit.

## 4. Docs, related fix and acceptance runs

- [ ] 4.1 `AGENTS.md` § How to verify a change and `README.md` § Testing: reuse, the record's place, `VOICEPUNCHER_REUSE_STAGES=0`; `justfile` comments; `copy-pool/` in `.claude/kit.yaml` `checks.build_inputs` and the AGENTS.md build input list (decision 6).
- [ ] 4.2 `just check` passes; `just full-check` on the same commit runs only `bundle`, `e2e-regression`, `e2e-seams`, `e2e-resilience` and its accuracy stage (plus `build`/`build-e2e`); stage tables in the PR.
- [ ] 4.3 A docs-only commit, then `just check`: every stage reused, no lock taken.
- [ ] 4.4 A scratch edit of a refinement adapter file (not committed): `just check` reruns `fmt`, `lint`, `test`, the builds, `e2e-smoke-refinement`, `refine-eval` and the stages with no narrower list, and reuses `e2e-smoke-voice`; a scratch failing test reruns from its stage on; revert.
- [ ] 4.5 Baseline from `just stage-times` (check-level runs per merged change, 14–18 Oct) in the PR; commit.
- [ ] 4.6 Version: no bump — test tooling only (proposal § Release); manual checks: none.
