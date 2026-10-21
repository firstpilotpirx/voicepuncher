# Design

## Context

`_stages` reuses a whole stage when its key (`_stage-keys`) has a pass record in `stage-passes/<stage>/<key>`. `_test` runs one nextest invocation over the workspace (`test_selection`), then the `justfile` self-tests (`_test-stage-keys`, `_test-lock-phases`, `_test-class-locks`). The workspace has four members: `app` (dev-depends on `e2e-test`), `command-spotting` (depends on `copy-pool`), `copy-pool`, `e2e-test` (depends on `app`, `command-spotting`, `copy-pool`).

## Goals / Non-Goals

**Goals:** when the `test` stage runs, it runs only the packages (and self-tests) whose inputs changed since a pass anywhere on this machine; the journal and `stage-times` show what reuse saved.

**Non-Goals:** test-level selection inside a package; splitting the model stages further; kit changes.

## Decisions

1. **Part keys.** `_stage-keys` takes, besides stage names, parts `test@<dir>`: for a workspace member `<dir>` (from the root `Cargo.toml` `members`) its key hashes the code-stage inputs (as `test`'s, `none` files left out) except the files under a member directory outside its scope; its scope is `<dir>` plus every member it reaches through `path = "../<member>"` dependencies in member `Cargo.toml` files, transitively (dev-dependencies included). `test@justfile` hashes the `justfile` alone (the self-tests read nothing else). `test@members` expands to one part per member. Toolchain and variables count as for any stage. Rejected: `cargo metadata` (slower, and needs a building workspace in the self-test's scratch repository).
2. **Selection in `_test`.** Outside a rerun, `_test` computes the parts' keys; with `VOICEPUNCHER_REUSE_STAGES` not `0`, a part whose key has a record in `stage-passes/test@<dir>/` prints `↺ test@<dir> reused: …` and does not run. The packages left run in one nextest invocation (`-p <name>…` with the E2E harness filter); none left → no nextest run. The self-tests run unless `test@justfile` is reused. After a pass each part that ran and whose key held through the run gets its record; the record is written by a shared recipe `_record-pass` that `_stages` uses too. A rerun (`VOICEPUNCHER_RERUN_FILTER`) keeps the whole-workspace selection, reuses and records nothing.
3. **Journal and summary.** A reused stage's `from` gains `seconds` (from its pass record). `stage-times` adds a `saved` column (sum of `from.seconds` over the reused runs of a stage) and a `reuse saved` total line.
4. **Accuracy after a command spotting edit.** Kept: `@accuracy` scenarios run command spotting (proposal § Decisions taken).

## Test levels

No spec scenario (`skip_specs: true`).

| Acceptance item | Level |
|---|---|
| part keys follow package scope; `justfile` edit changes only `test@justfile` among parts | recipe self-test `_test-stage-keys`, run by the `test` stage |
| second `just test` on an unchanged tree runs no package | recipe run with scratch `VOICEPUNCHER_STAGE_PASSES` / `VOICEPUNCHER_STAGE_JOURNAL`, `VOICEPUNCHER_REUSE_STAGES` passed through `_test` alone |
| `stage-times` shows saved time | recipe run on a scratch journal |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Risks / Trade-offs

- [A package reads a file of a member outside its scope (an `include_str!("../../other/…")`)] → a path dependency is the only cross-member reference today; a file outside every member feeds every part.
- [A part pass recorded under one `test_selection` filter is reused under another] → the filter is in the `justfile`, which feeds every part.
