# Design

## Context

See proposal.md § Why. `app/tests/`: 37 test files and 4 test directories, each a target; four are `harness = false` (the Gherkin harness, three macOS main-thread runners with identical `main()`s). `command-spotting/tests/`: 4 targets sharing `common/`; `e2e-test/tests/`: the harness and two helpers. Architecture tests R4 and R9 expect `tests/<stem>_it.rs`; baselines and `stage_inputs` name test files by path; nextest config and the justfile name binaries. nextest runs every test in its own process whatever the binary.

## Goals / Non-Goals

**Goals:** one link per member after an edit; every test keeps its group, priority, limit and a readable id; a test file no binary includes fails `just test` by name.

**Non-Goals:** test contents; the Gherkin harnesses; the `app ↔ e2e-test` dev-dependency cycle.

## Decisions

### D1: `tests/it/main.rs`, one module per former file, `_it` suffix dropped

`app/tests/it/main.rs` declares `mod <adapter>;` per former `<adapter>_it.rs` and per former directory (`it/recording_cpal.rs` + `it/recording_cpal/synthetic.rs`); `architecture`, `size_budgets` and `unsafe_impls` join it. Ids become `voicepuncher::it transcription_openai::should_…`. `main.rs` declares `mod support;` once (`it/support/`, the former `tests/support/`, holding only what several binaries share; `cucumber_integration.rs` reaches it by `#[path]`); a helper one module alone uses is that module's submodule (`it/textrefinement_openai/refinement_stub.rs`), so the guard sees it declared. Same for `command-spotting/tests/it/` (`engine`, `placed_commands`, `server`, `stall`, with `common`, `scripted_hearing`, `stream_clock`) and `e2e-test/tests/it/` (`confirmation_time`, `hang_probe`). Rejected: `autotests = false` over the old files — no moves, but a forgotten file becomes invisible to Cargo; after the move a stray `tests/*.rs` still builds and the guard names it.

### D2: one `harness = false` binary `main_thread` for the three macOS runners

One `main()` concatenates the `TESTS` tables of `visualfeedback`, `menubar`, `settings`, listing each test as `<module>::<name>`, so the CI profile still leaves out the overlay tests. libtest cannot run a test on the main thread, so they cannot join `it`.

### D3: filters select by module path inside the member's `it` binary

`binary(x_it)` → `binary(it) & package(<member>) & test(/^x::/)` in every nextest override and the CI default filter. The justfile's smoke, seams and placed-commands selections become module names with `--test it`; `_smoke-part-filter` emits `test(/^<module>::/)` terms; `stage_inputs` names `app/tests/it/<module>.rs`. `rerun-failed` needs nothing: ids carry the module path.

### D4: guard test in `it/test_layout.rs`

For every member with `tests/`: each `.rs` file or directory under `tests/it/` (and `tests/main_thread/`), at any depth, is declared in the file that owns its directory (`main.rs` at the top; `<dir>.rs` beside the directory or `<dir>/mod.rs` inside it below), and `tests/` holds only the harness roots, `it/`, `main_thread/`, `steps/`, `features/` and the size baseline (the shared helpers live in `it/support/`, reached by the Gherkin harness through `#[path]`); a violation names the file and the owner that leaves it out. Its own file beside `architecture.rs`, which is listed in the size baseline and must not grow; the walk and its unit test live in the submodule `it/test_layout/walk.rs`, so the listed directory `app/tests/it` does not grow either.

### D5: debug info by measurement

`[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false`, each kept only when `cargo build --workspace --tests --timings` after a touch is faster with it. Measured (task 4, load average 10–13): the moves alone 11 units, 11 s wall, 40.5 s CPU; with `line-tables-only` 5 s wall, 20.8 s and 20.1 s CPU in two runs — kept; with `debug = false` for the dependencies on top 5 s wall, 20.8 s CPU twice — no gain, dropped. Every open worktree rebuilds dependencies once after the merge (about 60 s).

### D6: acceptance measurement

Before the first move and after the last task: `touch app/src/core/domain/cleaned_text.rs`, then `cargo build --workspace --tests --timings`; the sum of unit durations, the wall clock and the load average go into proposal.md § Why; the test count from `just test`'s summary.

### D7: baselines move with the files

`just size-baseline` and `just analyzers-baseline` rewrite paths; a comparison by value after mapping `app/tests/<x>_it.rs` → `app/tests/it/<x>.rs` allows no grown or added measure except the directory lines the move creates: `app/tests` 7008 → `app/tests/it` 6274, and `command-spotting/tests/it` 2132 (its four former binaries and `main.rs` in one directory; the same lines as before, now over the directory budget together). Result (task 2.6 and 3.4): every moved entry keeps its value; the only other differences shrink (entries main's merge left stale). `.config/analyzers/recipes.toml` gains `shellcheck _it-filter SC2043` (a `for` over a recipe parameter, as `_lock-phases` and `_model-stages` already have).

## Dependencies

`fix-clear-text-please-unrecognized`: the `full-check` of this branch on 2026-10-22 failed in `e2e-regression` on the voice control scenario "Clear text is recognized as clear all text" ("clear text please" recognized as nothing), which this change does not touch; the flake rule files its fix and makes this branch wait for it before landing. `fix-slow-confirmation-of-russian-filler`: the second `full-check` (17:03, commit a2260faa) failed in `e2e-resilience` on "Russian speech that makes speech recognition repeat itself" (one confirmation of 2.116 s against the 2 s bound, load average 14–20), also untouched by this change; the same rule.

Otherwise none: `voice-commands-under-load` and `refresh-human-docs` share `.config/nextest.toml`, `AGENTS.md` and the baselines but rework nothing this change builds on; the one merged second moves `refresh-human-docs`'s `app/tests/docs_check/main.rs` into `it/docs_check.rs` (the guard names it); `voice-commands-under-load` merged first, and its `command-spotting/tests/under_load/` became `it/placed_commands/under_load/`, a submodule of `placed_commands` as before.

## Risks / Trade-offs

- [A filter typo selects nothing] → `_smoke-part-filter` self-tests, nextest's `--no-tests=fail` default, the test count in the bar.
- [`line-tables-only` drops variable values in a debugger] → panics keep file and line.
- [The machine is never idle during the orchestrator run] → the load average is recorded; the CPU sum is the primary figure.

## Test levels

| Spec scenario | Level |
|---|---|
| A one-file edit relinks one binary per member | measurement (D6, recorded in the proposal); the guard test (`it/test_layout.rs`) checks the layout that makes it so: every test file is a module of its member's `it` or `main_thread` binary |
| A test file the binary leaves out fails the checks | plain Rust, `it/test_layout.rs` (D4; its own file, so the listed `architecture.rs` does not grow) |
| A moved test keeps its isolation and limits | justfile self-test of `_smoke-part-filter`; `just test` and `just check` with the same test count |
