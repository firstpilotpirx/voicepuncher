## Why

A `full-check` takes 15–24 min under the machine lock whatever the branch changed, and `just test` takes ~2 min and ran 161 times on 2026-10-20/21 (5.6 h). The user's decision (audit of 2026-10-21): cache the checks "to the maximum", the way Bazel, Gradle, Nx and Turborepo do — a stage whose inputs did not change is not run again.

`reuse-passed-check-stages` (archived 2026-10-21) already landed most of it: every stage of `test`, `check`, `full-check` and the E2E recipes has a content hash of its inputs (`_stage-keys`, `stage_inputs`), a stage whose key has a pass record from any worktree of this machine is reused (`↺ reused`), the smoke set is split per capability so a change to one capability reuses the others, and a merge of `main` reruns only the stages whose inputs it changed. What is left:

- The `test` stage is one unit: an edit under `command-spotting/` alone reruns every test of `app/` (the largest crate), though no `app/` test reads it.
- `just stage-times` counts reused stages but not the time they saved, so nobody can tell what the cache is worth.

## What Changes

- The `test` stage keys each workspace package on its own: the files of the package, of every workspace package it depends on (dev-dependencies included) and every file outside the packages that feeds a code stage. When the stage itself is not reused, it runs only the packages whose key has no pass record (nextest `-p` filter); the others are reported `↺ reused` with the pass they came from. The `justfile` self-tests run only when the `justfile` changed. Each package that passed gets its pass record in `stage-passes/test@<dir>/`. Nothing changed → nothing runs. `VOICEPUNCHER_REUSE_STAGES=0` runs every package.
- A reused stage's journal entry carries the seconds of the pass it reused; `just stage-times` shows per stage the time reuse saved and the total saved over the period.
- `AGENTS.md` § How to verify a change says so.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`_stage-keys`, `_test`, `_stages` pass records, `_stage-journal-append`, `stage-times`, `_test-stage-keys`), `AGENTS.md` § How to verify a change.
Acceptance bar: `_test-stage-keys` proves that a file under `command-spotting/` changes the keys of `command-spotting` and `e2e-test` but not of `app` or `copy-pool`, a file under `copy-pool/` changes every package's key, a file outside the packages changes every key and a `justfile` edit alone changes the self-tests' key; `just test` twice on an unchanged tree runs no package the second time (the stage itself is then reused); `just stage-times` prints a saved column.
Out of scope: fixing flaky tests (`eradicate-flaky-tests`, archived); batch landing; the lock itself; kit changes (`kit verify` keeps reading the recipe's exit code).

## Capabilities

### Modified Capabilities

- none: test tooling only (check recipes and the stage journal); no product capability changes (`skip_specs`).

## Impact

- `justfile`, `~/.cache/voicepuncher/stage-times.jsonl` (reused stages carry `from.seconds`), `~/.cache/voicepuncher/stage-passes/test@<dir>/`, `AGENTS.md`.

## Decisions taken

- The name stays `reused` and the switch `VOICEPUNCHER_REUSE_STAGES=0`, as landed; no `cached` alias or `VOICEPUNCHER_NO_CACHE`.
- kit stays unchanged: a recipe whose stages are all reused exits 0 and counts as passed.
- `reuse-passed-check-stages` is already archived with its own implementation; nothing to supersede.
- A change under `command-spotting/` keeps running the accuracy stage: the `@accuracy` scenarios run command spotting and voice-command matching (`recognition_paths`), so skipping it would lose coverage; the original acceptance item "no `e2e-bench`" is dropped.
- The lock-hold-per-change target (below half the 2026-10-20/21 baseline) is measured after the batch lands, not a gate of this change.

## Release

SemVer: no release — the checks change, the product does not.
Target version: unchanged.
