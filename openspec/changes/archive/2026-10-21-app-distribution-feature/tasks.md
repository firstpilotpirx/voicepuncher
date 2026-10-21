## 1. The signing identity gate

- [x] 1.1 Red: unit tests in `e2e-test/src/scenario_filter.rs` for `Gate::signing_identity` (set opens, unset and empty close with a `GATED:` line); run them, see them fail
- [x] 1.2 Green: `Gate::signing_identity` in `e2e-test/src/scenario_filter.rs`, added to the gates of `e2e-test/tests/cucumber_e2e.rs`
- [x] 1.3 `just test` and commit

## 2. The app-distribution feature

- [x] 2.1 Red: `e2e-test/tests/features/app_distribution.feature`, one `Rule:` per requirement, every scenario of the spec (the manual ones `@wip` with the reason), steps in `e2e-test/tests/steps/app_distribution.rs`; run it with the `bundle` profile, see the checked scenarios fail
- [x] 2.2 Green: the step code, the `bundle` profile in `.config/nextest.toml`, `_check-bundle` split out of `_bundle` in `justfile`, `_bundle` running the `@bundle` scenarios, `_test-bundle-guard` removed
- [x] 2.3 `kit feature-lint` passes
- [x] 2.4 `just test` and commit

## 3. Version and manual checks

- [x] 3.1 Version unchanged (no release: tests only)
- [x] 3.2 Manual checks: none
