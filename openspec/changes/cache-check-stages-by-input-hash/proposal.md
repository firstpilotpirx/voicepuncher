## Why

A `full-check` takes 15–24 min under the machine lock whatever the branch changed, and `just test` takes ~2 min and ran 161 times on 2026-10-20/21 (5.6 h). Every run rebuilds the same verdict: a branch that touched only `command-spotting/` still runs the dictation E2E, the WER benchmark and the refinement evaluation; a merge of `main` that brought only documents or another crate still invalidates every passing stage, because `reuse-passed-check-stages` (filed, 0/14 tasks) keys the reuse on the commit SHA, not on what the stage reads. The user's decision (audit of 2026-10-21): cache the checks "to the maximum", the way Bazel, Gradle, Nx and Turborepo do — a stage whose inputs did not change is not run again.

## What Changes

- Every stage of `test`, `check` and `full-check` gets a content hash of its inputs: the files of the crates it tests (per `build_inputs` and the Cargo package graph), `.config/nextest.toml`, the `justfile` recipe text, the model files and their settings for the model stages, the corpus sample for `e2e-bench`. The stage journal records the hash with the result.
- A stage whose hash has a passing record on this machine is skipped and reported as `cached` (with the run it came from), in every recipe; a failing record is never reused. `VOICEPUNCHER_NO_CACHE=1` forces every stage.
- `just test` runs only the tests of the crates whose inputs changed since the last passing run and the crates that depend on them (nextest package filter over the workspace graph); nothing changed → nothing runs, verdict `cached`.
- The E2E stages pick their scenario sets from the changed paths through a path → capability-tag map in the `justfile` (as `recognition_paths` already picks the accuracy sample): a change that touches only voice control runs `@voice_control` and the seams, not the refinement evaluation; a change that touches shared code (`pipeline`, `main.rs`, wiring, `Cargo.lock` with a new dependency) runs everything.
- `kit sync-base` and `kit land` read the same cache: after a merge of `main`, the stages whose hash is unchanged carry over, so a docs-only or other-crate move costs nothing; this supersedes `reuse-passed-check-stages`, whose SHA-keyed reuse is a special case of the hash.
- `just stage-times` shows cached stages and the time they saved.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`_stages`, `test`, `check`, `full-check`, the E2E recipes, `stage-times`), `.config/nextest.toml`, the stage journal format, `.claude/kit.yaml` `checks` if kit must learn the cache.
Acceptance bar: a second `full-check` on an unchanged tree finishes in under 1 min with every stage `cached`; a change under `command-spotting/` only runs no dictation E2E and no `e2e-bench`; `just stage-times 1d` after a day of orchestrated changes shows lock hold per change below half the 2026-10-20/21 baseline (≈ 60 min per change).
Out of scope: fixing flaky tests (`eradicate-flaky-tests`); batch landing (`batch-landing`); the lock itself.

## Capabilities

### Modified Capabilities

- `test-infrastructure` (the capability the check recipes and the stage journal live under): stages are cached by input hash and selected by changed paths.

## Impact

- `justfile`, `.config/nextest.toml`, `~/.cache/voicepuncher/stage-times.jsonl` (new fields), `AGENTS.md` § How to verify a change, `openspec/changes/reuse-passed-check-stages` (superseded or narrowed).

## Open points

- Only "passed" is cached; a cached pass requires deterministic tests, so this change lands after `eradicate-flaky-tests` — proposed: `depends_on` it.
- Whether kit's check records (`kit verify`, `carryOver`) read the journal hash or the `justfile` reports `cached` as a pass to kit — proposed: the recipe exits 0 with `cached` stages and kit stays unchanged; a later agent-kit change may read the hash.
- Whether `reuse-passed-check-stages` is archived into this change or kept for the kit-side part — proposed: superseded, archived with a note.
- Target version: no release (tooling only) — proposed.

## Release

SemVer: no release — the checks change, the product does not.
Target version: unchanged.
