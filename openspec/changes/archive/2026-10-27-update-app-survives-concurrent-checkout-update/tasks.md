# Tasks

## 1. Update the checkout safely

- [x] 1.1 Red: add the Rule "The app update survives a concurrent update of the main checkout" with its six scenarios to `dev-tools/tests/features/change_workflow.feature`, steps in `dev-tools/tests/steps/checkout.rs` (scratch remote and clone, a held `index.lock` released by a thread that fast-forwards); run `just test-one 'scenario: Another program updates the checkout during the app update'`, see it fail; commit `test: reproduce update-app failing on a concurrent checkout update` (intentionally red)
- [x] 1.2 Green: add `dev-tools update-checkout` (`dev-tools/src/checkout.rs`, listed in `COMMANDS` of `dev-tools/src/main.rs`): fetch, then fast-forward in a loop of about 30 s, the stop lines of design.md; the six scenarios pass
- [x] 1.3 Red: unit test in `dev-tools/src/justfile_rules.rs` that `update-app` calls `update-checkout` and holds no `git pull`; see it fail
- [x] 1.4 Green: `update-app` in `justfile` calls `{{dev_tools}} update-checkout` (with the `_dev-tools` dependency) instead of `git switch main && git pull`
- [x] 1.5 `just test`, commit

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.41.1
- [x] 2.2 `just check`, commit
- [x] 2.3 Manual checks: none (`update-app` is no manual-run area; the scenarios drive `update-checkout` on scratch clones)
