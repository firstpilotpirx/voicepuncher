## Why

The audit of 2026-10-22 (five independent reviews over 74 634 lines of Rust, 1 408 tests) found that the architecture rules held — zero layer violations, every adapter with an integration test — but nothing limits the size and complexity of a unit, so three hubs grew past what a reader can hold: `app/src/core/application/pipeline.rs` (1 882 lines, `Pipeline` with 19 fields and 6 `pub` ports), `command-spotting/src/engine/confirming.rs` (1 297 lines, `apply_answer` 99 lines with ~10 flags, `JobContext` with 8 variants, `Session` with ~25 fields) and the `justfile` (2 390 lines). 22 production functions are over 50 lines, 3 over 100 (`main` 165, `live_watcher_thread` 108, `content` 104); the largest nesting is 8. The kit's `quality.md` § Size and complexity budgets and `enforcement-spec.md` R-130–R-135 state the budgets (function 40 code lines, file 300, parameters 5, nesting 4, cognitive complexity 15, public members 35) and name their clippy wiring, but this project never wired them: no `clippy.toml`, no `[lints]` table, no size script. The owner reads task descriptions, not code, so a budget nobody enforces is a budget nobody sees.

## What Changes

- The budgets become checks: `clippy.toml` with `too-many-lines-threshold`, `too-many-arguments-threshold`, `excessive-nesting-threshold`, `cognitive-complexity-threshold`; `[workspace.lints.clippy]` turning `too_many_lines`, `too_many_arguments`, `excessive_nesting`, `cognitive_complexity` and `type_complexity` on as errors; file length and public-member count checked by a rule of `app/tests/architecture.rs` (code lines, as the kit defines them).
- Budgets for structure too: a struct with more than 12 fields and a module directory over a stated budget fail the same rule, so a `Pipeline` or a `Session` cannot grow silently.
- A ratchet, not a big bang: the units over budget today are listed in one baseline file with their current measure; a check fails when a listed unit grows, when a new unit exceeds a budget, or when a listed unit shrinks under budget without leaving the list. Every change that touches a listed unit must shrink it (`AGENTS.md` § Workflow gets the rule, `/opsx:verify` checks it).
- `AGENTS.md` § Architecture rules names the budgets and the baseline; `rustfmt.toml` sets `struct_lit_width`/`struct_variant_width` as `quality.md` requires so line counts measure complexity, not formatting.
- `just lint` runs the size check with the clippy stage; `just test` runs the architecture rule as today.

Current stage: PoC. Target stage: PoC.
Scope: the thresholds, the baseline, the checks and the rule text; no code is split in this change (the splits are `slim-pipeline-module`, `tame-confirming-special-cases`, `stage-runner-in-rust`).
Acceptance bar: `just lint` and `just test` fail on a function, file, struct, parameter list, nesting or complexity over budget outside the baseline, and on a baseline unit that grew; the baseline names every unit over budget on `main` with its measure; the architecture test proves each rule with a fixture.
Out of scope: shrinking the hubs; `justfile` size (its own change); tests under budget (the kit applies budgets to tests too — kept as an open point).

## Capabilities

### Modified Capabilities

- `change-workflow`: a change that touches a baseline unit must shrink it; the check that proves it.

## Impact

- `clippy.toml`, `rustfmt.toml`, `Cargo.toml` (`[workspace.lints]`), `app/tests/architecture.rs` (new rules R14–R16 and the baseline file `app/tests/size_baseline.toml`), `justfile` (`_lint`), `AGENTS.md`.

## Open points

- Thresholds: the kit's numbers (40 / 300 / 5 / 4 / 15 / 35) as the targets, or the project's own looser first step (60 / 600 / 6 / 5 / 20 / 35) with the kit's as the second ratchet? Proposed: the kit's numbers from the start, with the baseline carrying the gap — a looser number is a second baseline.
- Whether the budgets apply to test code from the start (the kit says yes). Proposed: production first, tests in the baseline with a date.
- Where the struct-field and directory budgets come from (the kit has none). Proposed: 12 fields, 2 000 code lines per directory, stated in `AGENTS.md` with the reason.

## Release

SemVer: no release — tooling and rules only, no behavior change.
Target version: 0.35.0 (unchanged).
