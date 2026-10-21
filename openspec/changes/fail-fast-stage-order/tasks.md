# Tasks

No spec scenario (`skip_specs: true`): order and fail-fast are tested by recipe self-tests and one unit test (design.md § Test levels).

## 1. Stage order (decisions 1, 2)

- [ ] 1.1 Red: `_test-lock-phases` expects the phases of `check_model_stages` and `full_check_model_stages` in the new order, the same stage set as before, and the voice control part's binaries `placed_commands_it` and `voice_forms_it` first; run it, it fails.
- [ ] 1.2 Green: `check_model_stages`, `full_check_model_stages`, `e2e_smoke_stages` and the voice control part's binary order in the `justfile`; `check` and `full-check` use the variables; `just test`; commit.

## 2. Fail-fast in check and full-check (decisions 3, 4)

- [ ] 2.1 Red: unit test in `e2e-test/src/harness_args.rs` for adding `--fail-fast` to the cucumber command line only when the variable is `1`; `_test-stage-keys` expects `VOICEPUNCHER_FAIL_FAST` to change no key; run both, they fail.
- [ ] 2.2 Green: the harness function and its call in `e2e-test/tests/cucumber_e2e.rs`; `VOICEPUNCHER_FAIL_FAST` volatile in `_stage-keys`; `check` and `full-check` set it; the model stages' nextest calls add `--fail-fast` when it is set; `just test`; commit.

## 3. Docs

- [ ] 3.1 `AGENTS.md` § How to verify a change: the stage order and fail-fast of `check` and `full-check`; `justfile` comments; commit.
- [ ] 3.2 Version: no bump — test tooling only (proposal § Release); manual checks: none.
