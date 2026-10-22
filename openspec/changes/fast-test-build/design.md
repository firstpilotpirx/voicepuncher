# Design

## Context

See proposal.md § Why. `app/tests/`: 37 test files and 4 test directories, each a target; four are `harness = false` (the Gherkin harness, three macOS main-thread runners with identical `main()`s). `command-spotting/tests/`: 4 targets sharing `common/`; `e2e-test/tests/`: the harness and two helpers. Architecture tests R4 and R9 expect `tests/<stem>_it.rs`; baselines and `stage_inputs` name test files by path; nextest config and the justfile name binaries. nextest runs every test in its own process whatever the binary.

## Goals / Non-Goals

**Goals:** one link per member after an edit; every test keeps its group, priority, limit and a readable id; a test file no binary includes fails `just test` by name.

**Non-Goals:** test contents; the Gherkin harnesses; the `app ↔ e2e-test` dev-dependency cycle.

## Decisions

### D1: `tests/it/main.rs`, one module per former file, `_it` suffix dropped

`app/tests/it/main.rs` declares `mod <adapter>;` per former `<adapter>_it.rs` and per former directory (`it/recording_cpal.rs` + `it/recording_cpal/synthetic.rs`); `architecture`, `size_budgets` and `unsafe_impls` join it. Ids become `voicepuncher::it transcription_openai::should_…`. `main.rs` declares `#[path = "../support/mod.rs"] mod support;` once. Same for `command-spotting/tests/it/` (`engine`, `placed_commands`, `server`, `stall`, with `common`, `scripted_hearing`, `stream_clock`) and `e2e-test/tests/it/` (`confirmation_time`, `hang_probe`). Rejected: `autotests = false` over the old files — no moves, but a forgotten file becomes invisible to Cargo; after the move a stray `tests/*.rs` still builds and the guard names it.

### D2: one `harness = false` binary `main_thread` for the three macOS runners

One `main()` concatenates the `TESTS` tables of `visualfeedback`, `menubar`, `settings`, listing each test as `<module>::<name>`, so the CI profile still leaves out the overlay tests. libtest cannot run a test on the main thread, so they cannot join `it`.

### D3: filters select by module path inside the member's `it` binary

`binary(x_it)` → `binary(it) & package(<member>) & test(/^x::/)` in every nextest override and the CI default filter. The justfile's smoke, seams and placed-commands selections become module names with `--test it`; `_smoke-part-filter` emits `test(/^<module>::/)` terms; `stage_inputs` names `app/tests/it/<module>.rs`. `rerun-failed` needs nothing: ids carry the module path.

### D4: guard test in `it/architecture.rs`

For every member with `tests/`: each `.rs` directly under `tests/it/` is declared in its `main.rs`, and `tests/` holds only the harness roots, `it/`, `main_thread/` and the helper directories; a violation names the file.

### D5: debug info by measurement

`[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false`, each kept only when `cargo build --workspace --tests --timings` after a touch is faster with it. Every open worktree rebuilds dependencies once after the merge.

### D6: acceptance measurement

Before the first move and after the last task: `touch app/src/core/domain/cleaned_text.rs`, then `cargo build --workspace --tests --timings`; the sum of unit durations, the wall clock and the load average go into proposal.md § Why; the test count from `just test`'s summary.

### D7: baselines move with the files

`just size-baseline` and `just analyzers-baseline` rewrite paths; a comparison by value after mapping `app/tests/<x>_it.rs` → `app/tests/it/<x>.rs` allows no grown or added measure except the directory line that moves from `app/tests` to `app/tests/it`.

## Dependencies

None: `voice-commands-under-load` and `refresh-human-docs` share `.config/nextest.toml`, `AGENTS.md` and the baselines but rework nothing this change builds on; the one merged second moves `refresh-human-docs`'s `app/tests/docs_it.rs` into `it/docs.rs` (the guard names it).

## Risks / Trade-offs

- [A filter typo selects nothing] → `_smoke-part-filter` self-tests, nextest's `--no-tests=fail` default, the test count in the bar.
- [`line-tables-only` drops variable values in a debugger] → panics keep file and line.
- [The machine is never idle during the orchestrator run] → the load average is recorded; the CPU sum is the primary figure.

## Test levels

| Spec scenario | Level |
|---|---|
| A one-file edit relinks one binary per member | measurement (D6); the guard test checks `cargo nextest list` shows one `it` per member |
| A test file the binary leaves out fails the checks | plain Rust, `it/architecture.rs` (D4) |
| A moved test keeps its isolation and limits | justfile self-test of `_smoke-part-filter`; `just test` and `just check` with the same test count |
