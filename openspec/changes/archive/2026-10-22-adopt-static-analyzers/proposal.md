## Why

The audit of 2026-10-22 found the code clean under the default clippy with `-D warnings` and nothing else: clippy `pedantic` + `nursery` report ≈ 2 060 findings over all targets, among them 131 `&mut` parameters never mutated, 103 parameters passed by value and not consumed, ≈ 300 lossy `f32`/`usize` casts in audio and UI code, 314 struct-name repetitions, 45 redundant closures — the signature of generated code that nobody tightens. No tool measures cyclomatic or cognitive complexity, duplication, dead dependencies, license or advisory state, spelling, or the shell inside the `justfile`. The kit's `tech-matrix-rust.md` lists clippy `pedantic` as the linter and `cargo audit` as the scan, but the project runs neither; the owner asked for every static analyzer that can hold the line, cyclomatic complexity included, so that entropy is caught by a tool, not by a reader.

## What Changes

- clippy: a second, ratchet run of clippy with `pedantic`, `nursery` and the useful `restriction` lints, whose groups, lints and justified exceptions (each with its reason) live in `.config/analyzers.toml`, beside the strict `-D warnings` run; the current ≈ 2 060 findings fixed or recorded in a baseline that may only shrink (the ratchet of `enforce-size-and-complexity-budgets`).
- Complexity: `rust-code-analysis-cli` (Mozilla) reports cyclomatic and cognitive complexity and length per function (Halstead and the maintainability index are left out: no gate reads them); `just metrics` prints the worst 25 and a per-module table (function count, median and max length, max cyclomatic complexity, unsafe, unwrap, clones, fan-in/fan-out of the module graph, import cycles) and fails when a baseline number grows; cognitive complexity gates through clippy (the budgets change), cyclomatic is reported and ratcheted.
- Duplication: a copy-paste detector over Rust (`jscpd` with the Rust tokenizer, or an equivalent) with a threshold and a baseline; the known duplicates (`request_agent` in two adapters, `to_mono` and the rubato parameters in two crates, the window builders of two windows) are its first catches.
- Dependencies: `cargo-machete` (unused), `cargo-deny` (advisories, licenses, duplicate crate versions, sources — the app runs offline, so the scan runs on the developer machine in `just lint`), `cargo-semver-checks` for `copy-pool` as a library.
- Unsafe: the project's own `unsafe` count per crate, in the metrics baseline (`cargo-geiger` fails on this workspace; dependency unsafe is out of scope).
- Text and shell: `typos` over the repository; `shellcheck` over every recipe body of the `justfile` (`just --dump` or a script that extracts them) and over the Perl recipes with `perlcritic`.
- Every tool pinned to an exact version (`exact-versions.md`), installed by one `just tools` recipe (shellcheck, from Homebrew, is version-checked and named, not installed) with a `_require-tools` guard that names the install command, never installing on its own; every analyzer runs in `just lint` without the machine lock; `just metrics` runs in `just test`.
- `AGENTS.md` § How to verify a change names each analyzer and what it gates.
- The advisories `cargo deny` found are fixed within compatible versions (`anyhow`, `rustls`, `webpki-roots`); the `zip` ones, a build dependency of `sherpa-onnx-sys`, are ignored with a reason; `cargo-geiger` is replaced by the project's own `unsafe` count (it fails on this workspace).

Current stage: PoC. Target stage: PoC.
Scope: the analyzers, their configuration, their baselines, the recipes and the rule text; the findings that are mechanical to fix (closures, by-value parameters, `&mut`, struct-name repetition) are fixed here; findings that need design (casts in audio code, cognitive complexity of the hubs) go into the baseline for the refactoring changes.
Acceptance bar: `just lint` runs every analyzer and fails on a new finding; `just metrics` prints the table and fails on a grown baseline; the baselines name every finding left; `just tools` installs the pinned set on a fresh machine.
Out of scope: dynamic analysis (miri, sanitizers, mutation testing, coverage — `measure-test-strength`), CI (`add-ci-smoke`).

## Capabilities

### Modified Capabilities

- `change-workflow`: the analyzers and the metrics ratchet as stages of `just lint` and `just test`.

## Impact

- `Cargo.toml` (`[workspace.lints]`), `clippy.toml`, `deny.toml`, `.typos.toml`, `.jscpd.json` (or equivalent), `justfile` (`tools`, `_require-tools`, `_lint`, `metrics`), a metrics script under `e2e-test/src` or a small workspace tool, baselines under `app/tests/` or `.config/`, `AGENTS.md`; mechanical fixes across `app/`, `command-spotting/`, `copy-pool/`, `e2e-test/`.

## Open points

- Which `restriction` lints to turn on (candidates: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `float_cmp_const`, `undocumented_unsafe_blocks`, `missing_docs_in_private_items`); proposed: `undocumented_unsafe_blocks` (with `enforce-unsafe-safety-comments`), `unwrap_used`/`expect_used`/`panic` as errors outside tests with the baseline, no `missing_docs_in_private_items`.
- Whether the metrics tool is a script in the repository or a `kit metrics` command shared by every project (`kit-placement.md`); proposed: `kit metrics` filed in agent-kit, this change calls it and keeps the baselines here.
- The duplication detector: `jscpd` needs Node; an alternative without Node (a token-hash script in the metrics tool) is proposed when `jscpd` proves awkward on this machine.

## Release

SemVer: no release — tooling only.
Target version: 0.36.1 (unchanged).
