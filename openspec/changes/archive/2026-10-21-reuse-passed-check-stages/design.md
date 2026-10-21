# Design

## Context

`_stages` runs unlocked stages, then model stages under the machine lock (`_model-stages`), and journals each run (`reused` is a reserved stage result). speed-up-tests adds `e2e-regression`; this change builds on its `justfile`.

## Goals / Non-Goals

**Goals:** a stage runs only when an input it depends on changed since a pass anywhere on this machine; a check whose model stages are all reused never takes the machine lock.

**Non-Goals:** no kit change (a run's result stays `passed`); no pass from another machine.

## Decisions

1. **A stage's key is a hash of its inputs.** `_stage-keys <stages…>` (Perl) prints a sha256 per stage over: its name; path and working-tree content hash of each input file, untracked ones included; the toolchain (rustc, cargo, nextest, just, macOS build, which fixes the `say` voices); the `VOICEPUNCHER_*` and `CUCUMBER_*` variables and the build and test settings among `RUSTFLAGS`, `RUSTDOCFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `RUSTC_WRAPPER`, `CARGO_BUILD_*`, `CARGO_PROFILE_*`, `CARGO_TARGET_*` and `NEXTEST_PROFILE`/`_RETRIES`/`_TEST_THREADS`, but the volatile ones (journal, report, ports, lock, colour), for every stage, so `VOICEPUNCHER_HANG_PROBE=1 just test` never reuses a pass; for model stages also size and mtime of the models, corpus, voice samples and service binaries. Rejected: the commit id (wrong for uncommitted edits and doc-only commits).
2. **Which files feed which stage.** A `justfile` table `stage_inputs` (`glob stage[,stage]` per line) names the model stages a file feeds; `code` means unlocked stages only, `none` no stage (`openspec/`, `docs/`, `.claude/`, Markdown no code includes). Unlocked stages take every file but `none`; a file no line names feeds every stage, so a new file is never skipped. Version-only Cargo edits do not count for model stages (as in `_accuracy-stage`); they do for `test` and `bundle`.
3. **`e2e-smoke` split by capability.** Four stages, each a nextest profile with its tag filter and limit: `e2e-smoke-voice`, `-refinement`, `-services` and `-speech` (with every capability that has no part of its own: a separate 6 s part for the dictation archive cost more than it saved), each with its capability's `*_it` binaries. `just e2e-smoke` runs all four. Rejected: parts inside one stage (a second level in table, journal and reuse).
4. **Reuse in `_stages`.** A stage whose key has a pass record shows `↺ reused` with its source and does not run. `build` and `build-e2e` run whenever a model stage runs (they fill this worktree's `target/`). With every model stage reused, no lock and no service. After a pass the key is computed again; the record is written only when it held. `VOICEPUNCHER_REUSE_STAGES=0` runs everything.
5. **The record.** `~/.cache/voicepuncher/stage-passes/<stage>/<key>` (`VOICEPUNCHER_STAGE_PASSES`): commit, branch, worktree, finish time, seconds; written by temp file and rename, kept at any age. The journal marks the stage `reused` with `from`; `just stage-times` counts reused stages apart, out of the times.
6. **Related fix.** `copy-pool/` (a workspace member) is missing from `.claude/kit.yaml` `checks.build_inputs` and the AGENTS.md build inputs: added.

## Test levels

No spec scenario (`skip_specs: true`).

| Acceptance item | Level |
|---|---|
| key changes with an input of the stage, not with a `none` file or another model stage's file; deleted and untracked files count | recipe self-test `_test-stage-keys` in a scratch git repository, run by the `test` stage |
| `check` then `full-check` on one commit runs only the full check's own stages | recipe run (stage table, journal) |
| after a docs-only commit no model stage runs and the lock is not taken | recipe run |
| an edit to a refinement file reruns the refinement stages and reuses the voice ones | recipe run |
| a failed stage reruns from that stage on | recipe run (scratch failing test, not committed) |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Risks / Trade-offs

- [A wrong `stage_inputs` line skips a needed stage] → unnamed files feed every stage; `VOICEPUNCHER_REUSE_STAGES=0` is available.
- [Four smoke runs lose concurrency] → measured: 481 s for five parts against 411–440 s in one run; the 6 s part merged back; the rest of the cost is paid back by any change that reuses the 307 s voice control part.
- [An input outside the repository is missed] → the `justfile` comment names `VOICEPUNCHER_REUSE_STAGES=0` for such changes.

## Dependencies

Stacks on speed-up-tests (declared): it changes the stage lists and adds `e2e-regression`.
