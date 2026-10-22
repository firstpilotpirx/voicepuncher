## Context

`just lint` runs clippy with `-D warnings` and the size budgets of `enforce-size-and-complexity-budgets` (denied in `[workspace.lints]`, ratcheted in `app/tests/size_baseline.toml`). Nothing else looks at the code: clippy `pedantic` + `nursery` report ≈ 4 500 findings over all targets (≈ 1 500 of them `unwrap_used` in tests), and no tool measures cyclomatic complexity, duplication, dependencies, spelling or the shell of the `justfile`.

## Goals / Non-Goals

**Goals:** every analyzer of the proposal runs in `just lint` or `just metrics`, pinned; every finding left is in a baseline that may only shrink; the mechanical findings are fixed.

**Non-Goals:** fixing the findings that need design (casts in audio code, `unwrap` in production paths, the complex hubs) — they stay in the baseline for the refactoring changes; dynamic analysis; CI.

## Decisions

### One workspace tool, `analyzers/`

A new workspace member `analyzers/` (binary `voicepuncher-analyzers`, not shipped) runs the analyzers that need a ratchet and owns their baselines: `clippy`, `recipes` (shellcheck, perlcritic), `metrics` (rust-code-analysis, module graph, duplication), each with `--write-baseline`. The analyzers without a ratchet (`typos`, `cargo machete`, `cargo deny`, `cargo semver-checks`) run straight from `_lint` with zero tolerance and their own configuration files (`.typos.toml`, `deny.toml`, `[package.metadata.cargo-machete]`).

*Alternative:* a `kit metrics` command in agent-kit (proposal's open point). Rejected for now: this session cannot change agent-kit, and the baselines and thresholds are project values; the tool is small enough to move into the kit later.

### The clippy ratchet is a second clippy run

The strict run (`cargo clippy -- -D warnings`) stays as it is, so the default lints and the size budgets keep zero tolerance and every other tool that runs clippy keeps working. The ratchet run adds the groups and lints of `.config/analyzers.toml` as `-W` flags, without `-D warnings`, reads `--message-format=json` and counts the findings per `<lint> <file>`; `.config/analyzers/clippy.toml` is the baseline. Lints switched off for the project are listed in the same file with their reason and passed as `-A`:

- `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`, `return_self_not_must_use`: documentation lints for a published library; this is an application.
- `missing_const_for_fn`: `const` on every eligible function freezes signatures without a caller that needs it.
- `redundant_pub_crate`: conflicts with the project's `pub(crate)` style inside private modules.
- `option_if_let_else`, `significant_drop_tightening`: nursery lints with known false positives.

The restriction lints `unwrap_used`, `expect_used` and `panic` are counted outside test code only: `allow-*-in-tests` in `clippy.toml`, and the test trees (`*/tests/`, `e2e-test/`) listed as `test_paths` in `.config/analyzers.toml`, where those three lints are not counted.

### Cucumber step signatures are exempt

`needless_pass_by_ref_mut` and `needless_pass_by_value` fire on ≈ 200 cucumber steps, whose signatures cucumber fixes (`&mut World`, captures by value). The `steps` modules of both harnesses carry `#[allow(…, reason = …)]` for these two lints; production code is fixed.

### Mechanical fixes by `cargo clippy --fix`

The machine-applicable lints the proposal names as mechanical (`use_self`, `redundant_closure_for_method_calls`, `semicolon_if_nothing_returned`, `doc_markdown`, `cast_lossless`, `map_unwrap_or`, `redundant_clone`, the small style lints) are fixed with `cargo clippy --fix`, then `cargo fmt`. `suboptimal_flops` (`mul_add`) changes the rounding of audio math and `unchecked_duration_subtraction` would add `unwrap`s: both stay in the baseline.

### Metrics

`rust-code-analysis-cli -m -O json` gives per-function cyclomatic and cognitive complexity and length; the tool prints the worst 25 by cyclomatic and one row per module (directory of `.rs` files): functions, median and max length, max cyclomatic, `unsafe`/`.unwrap()`/`.clone()` counts, fan-in and fan-out over `use crate::…` edges. Ratcheted (`.config/analyzers/metrics.toml`): every function over cyclomatic 10 with its value, the import cycles, `unsafe` per crate, and the duplicated lines `jscpd` reports over the Rust sources (min 50 tokens). Cognitive complexity stays gated by clippy (the budgets change). `just test` runs `metrics` as a stage of its own after `test`.

### `cargo-geiger` is replaced by the own count

`cargo geiger` 0.14 does not support Cargo workspaces with path dependencies well and scans every dependency for minutes; the unsafe count that is actionable here is the project's own, which `metrics` ratchets per crate. Dependency unsafe stays out of scope.

### Tools pinned and installed by `just tools`

`just tools` installs the pinned set (`cargo +stable install --locked <crate>@<version>`, `npm install -g jscpd@<version>`, `cpan` for Perl::Critic); `_require-tools` checks each version and prints the install command, installing nothing. Versions: cargo-machete 0.9.2, cargo-deny 0.20.2, cargo-semver-checks 0.51.0, typos-cli 1.51.1, rust-code-analysis-cli 0.0.25, jscpd 4.0.5, shellcheck 0.11.0 (Homebrew; checked, not installed), Perl::Critic 1.156.

## Test levels

| Scenario | Level |
|---|---|
| A new clippy finding fails the lint stage | unit (`analyzers` baseline and clippy parse) |
| A listed finding that shrank keeps passing | unit (`analyzers` baseline) |
| A finding that is gone must leave the baseline | unit (`analyzers` baseline) |
| A shell mistake in a recipe fails the lint stage | unit (`analyzers` recipes parse) and `just lint` |
| A missing analyzer names its install command | `just lint` on this machine (`_require-tools`) |
| A new complex function fails the metrics | unit (`analyzers` metrics parse and baseline) |
| Copy-pasted code fails the metrics | unit (`analyzers` jscpd parse and baseline) |
| The report lists the worst functions and the modules | unit (`analyzers` report) and `just metrics` |

## Risks / Trade-offs

- [The `use_self` fix touches hundreds of files and conflicts with parallel branches] → a conflict there is resolved by taking either side and rerunning `cargo clippy --fix`; noted for the batch.
- [The lint stage gets slower: a second clippy run, semver-checks builds rustdoc twice] → all unlocked; measured in the stage journal.
- [Baselines must be rewritten after merges] → `just analyzers-baseline`, reviewed like code, as `just size-baseline`.

## Dependencies

`enforce-size-and-complexity-budgets` (archived on `main`) supplies `clippy.toml`, `[workspace.lints]` and the ratchet pattern.
