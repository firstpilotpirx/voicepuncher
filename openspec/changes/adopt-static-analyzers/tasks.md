## 1. Ratchet baseline and the analyzers tool

- [ ] 1.1 Red: unit tests for the baseline comparison — a new key and a grown count fail, a shrunk count passes, a key gone fails as stale
- [ ] 1.2 Green: workspace member `analyzers/` with the `Baseline` type (TOML load/save, compare) and the CLI skeleton; `analyzers` added to the size budgets' members
- [ ] 1.3 `just test`; commit

## 2. Clippy ratchet

- [ ] 2.1 Red: unit tests that count clippy JSON messages per `<lint> <file>` and skip restriction lints under the test paths
- [ ] 2.2 Green: `voicepuncher-analyzers clippy` with `.config/analyzers.toml` (groups, lints, allowed lints with reasons, test paths); `clippy.toml` `allow-*-in-tests`
- [ ] 2.3 Mechanical fixes with `cargo clippy --fix`; `allow` of the two signature lints on the cucumber `steps` modules; fixes of `needless_pass_by_value` and `needless_pass_by_ref_mut` in production code
- [ ] 2.4 Baseline `.config/analyzers/clippy.toml` written
- [ ] 2.5 `just test`; commit

## 3. Recipes

- [ ] 3.1 Red: unit tests that turn `just --dump --dump-format json` recipes into shell and Perl scripts (interpolations replaced) and parse shellcheck and perlcritic findings into keys
- [ ] 3.2 Green: `voicepuncher-analyzers recipes`; baseline `.config/analyzers/recipes.toml`
- [ ] 3.3 `just test`; commit

## 4. Metrics

- [ ] 4.1 Red: unit tests for the rust-code-analysis JSON parse, the module table (median, fan-in/out, cycles) and the jscpd report parse
- [ ] 4.2 Green: `voicepuncher-analyzers metrics` printing the worst 25 and the module table and ratcheting cyclomatic > 10, cycles, unsafe per crate and duplicated lines; baseline `.config/analyzers/metrics.toml`
- [ ] 4.3 `just test`; commit

## 5. Recipes, configuration and docs

- [ ] 5.1 `just tools`, `_require-tools`, `_lint` running every analyzer, `metrics`, `_metrics` as a stage of `test`, `check` and `full-check`, `analyzers-baseline`
- [ ] 5.2 `.typos.toml`, `deny.toml`, cargo-machete metadata; findings of typos, machete and deny fixed
- [ ] 5.3 `AGENTS.md` § How to verify a change names each analyzer and what it gates
- [ ] 5.4 Keep `version` at 0.36.1 (no release: tooling only)
- [ ] 5.5 `just test`, `cargo fmt --all`, `just lint` pass; commit
