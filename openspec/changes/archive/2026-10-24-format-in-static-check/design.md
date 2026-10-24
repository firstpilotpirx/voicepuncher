## Context

`_static` (justfile) runs `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the size analyzer and the changed-file metrics under `set -euo pipefail`. A formatting slip stops it before the size check, so one more run is needed to see the next finding. The justfile is at 798 of its 800 lines (`dev-tools/src/justfile_rules.rs`), so the fix logic cannot live in the recipe.

## Goals / Non-Goals

**Goals:** a local `static` applies every mechanical fix first, names the files it changed, then runs the unchanged strict checks; CI and the landing queue's `full-check` change nothing and stay strict.

**Non-Goals:** fixing size budgets or metrics; changing which lints run; moving `typos` (a `lint` stage tool) into `static`.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Where the fixes live | a new `dev-tools static-fix fmt\|clippy` command in place of the two strict commands of `_static` (same line count) | the justfile has 2 lines left; Rust is tested by unit tests | shell in the recipe |
| Formatting, local | `cargo fmt --all -- -l`: formats in place and prints each file it changed | `fmt` takes under a second, so it always runs | `--check` then a fix run |
| Clippy, local | `cargo clippy --workspace --all-targets -- -D warnings` first, output captured; only when it fails, `cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged --allow-no-vcs`, then `cargo fmt -l` (a fix may leave code unformatted), then the same strict clippy again, its output shown | `clippy --fix` re-checks every workspace crate (measured 28 s on a warm tree against 0 s for a cached check), so it runs only when there is something to fix; the flags let it run on an uncommitted tree and in a scratch crate | `clippy --fix` on every run (+28 s on a clean tree) |
| When it is strict | `CI` set and non-empty, or `VOICEPUNCHER_STAGES_RECIPE=full-check` | hosted CI sets `CI`; `full-check` runs only in the landing queue, whose edits never land, so it must fail instead | a new flag `kit land` would have to pass |
| What the strict run does | `cargo fmt --all -- --check` and `cargo clippy … -D warnings`, exactly as today, after one line `● strict (<reason>): no fixes` | CI and the landing queue fail on what is left | fixing there too |
| Naming the changed files | each path `cargo fmt -l` prints and each `Fixed <path>` line `cargo clippy --fix` prints, as `  fixed <path>` | the tools already name what they changed; no git needed | a `git status` diff (the scratch crate has no git; unrelated dirty files would show) |
| Clippy's output, local | the first check's and the fix run's output printed only when the fix run fails (a compile error), so the error is named and `static` stops; otherwise the rerun's output alone | a finding that got fixed is noise | streaming every run |
| The stage key after a fix | nothing new: the stage runner already records no pass when what a stage reads changed during the run | `dev-tools/src/stages/mod.rs` prints "what it reads changed during the run: no pass recorded" | recomputing the key after the fixes |

## Tests

| Scenario | Level |
|---|---|
| A function over budget fails the static check without a test | slice (`dev-tools/tests/features/change_workflow.feature`, unchanged) |
| A compile error fails the static check | slice, the `_static` body run on a scratch crate (unchanged) |
| A new complex function in a changed file fails the static check | slice (unchanged) |
| The test recipe stops at the static check | slice (unchanged) |
| A clean tree passes the static check in seconds | slice, the recipe's order now `static-fix fmt`, `static-fix clippy`, size, changed metrics |
| A local static check fixes formatting and clippy findings and goes on | slice, the `_static` body run on a scratch crate with the prebuilt `dev-tools`; unit tests in `dev-tools/src/static_fix.rs` for the mode and the file names read from the tools' output |
| A strict static check fixes nothing | slice, the same scratch crate with `CI=true`; unit test for the mode |

## Risks / Trade-offs

- A run with a clippy finding costs the fix run and a second check (~30 s); a clean run costs nothing extra.
- A wrong machine-applicable fix is rare; the edit shows in the printed list and in the diff, reviewed like any other edit.
