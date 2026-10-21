# Design

## Context

Since this change was filed, `reuse-passed-check-stages` split the smoke set into four stages (`e2e-smoke-speech`, `-refinement`, `-voice`, `-services`) and `split-machine-lock` runs the model stages in two phases, `light` (voice control: `e2e-smoke-voice`, `e2e-regression`, `e2e-resilience`) and `heavy` (the rest), each phase in the order of its first stage, each keeping its stages' order. Today `check` and `full-check` list the speech part first, so the heavy phase runs first and the voice control part, which held most failures of the last 7 days (`placed_commands_it` 32, the voice control Gherkin scenarios), runs last. Inside the voice control part `placed_commands_it` runs after the three service binaries.

Measured medians in `check` (`just stage-times 7d`): `e2e-smoke-refinement` 15 s, `e2e-smoke-speech` 38 s, `e2e-quick` 44 s, `refine-eval` 56 s, `e2e-smoke-services` 83 s, `e2e-smoke-voice` 285 s; in `full-check` also `e2e-bundle` 31 s, `e2e-regression` 21 s, `e2e-resilience` 174 s, `e2e-bench` 221 s, `e2e-seams` 352 s.

## Decisions

1. **Fixed order, light phase first.** `check` runs `e2e-smoke-voice`, then `e2e-smoke-refinement refine-eval e2e-smoke-speech e2e-quick e2e-smoke-services`; `full-check` runs `e2e-smoke-voice e2e-regression e2e-resilience`, then `e2e-smoke-refinement refine-eval e2e-bundle e2e-smoke-speech <accuracy> e2e-smoke-services e2e-seams`. The light phase goes first because it holds the most failures and its first binary shows them within a minute; inside each phase the short stages come first and the long Gherkin runs (`e2e-smoke-services`, `e2e-seams`) last. The stage lists live in `justfile` variables (`check_model_stages`, `full_check_model_stages`), so the self-test reads the real order. Rejected: interleaving light and heavy stages (a third phase re-queues for a lock and restarts the services, minutes under contention); an order learned from the journal (revisited once the flaky tests are gone).
2. **Voice control part: most failing first.** `placed_commands_it` runs first, then `voice_forms_it`, `voice_samples_it`, `commandspotting_websocket_it`, then the Gherkin scenarios. The smoke parts' order in `e2e_smoke_stages` follows decision 1, so standalone `e2e-smoke` and `e2e` fail as early.
3. **Stop at the first failure inside `check` and `full-check` only.** Both recipes set `VOICEPUNCHER_FAIL_FAST=1`; every nextest call of a model stage adds `--fail-fast` when it is set, and the E2E harness adds cucumber's `--fail-fast` to its command line when it is `1`. Standalone recipes and `rerun-failed` keep reporting every failure, which helps when debugging. A passing check still runs every test: fail-fast only cuts a run that already failed.
4. **The flag changes no stage key.** `VOICEPUNCHER_FAIL_FAST` joins the volatile variables of `_stage-keys`: a pass under fail-fast ran every test, so it is as good as a pass of a standalone recipe, and the two keep reusing each other.

## Test levels

No spec scenario (`skip_specs: true`): test tooling only.

| Acceptance item | Level |
|---|---|
| `check` and `full-check` phases: light first, short stages before the long Gherkin runs, the same stages as before | recipe self-test `_test-lock-phases` on the real stage variables, run by the `test` stage |
| the voice control part runs `placed_commands_it` and `voice_forms_it` first | recipe self-test `_test-lock-phases` reading the part's binary order |
| `VOICEPUNCHER_FAIL_FAST` changes no stage key | recipe self-test `_test-stage-keys` |
| the harness adds `--fail-fast` only for `1` | unit test in `e2e-test/src/harness_args.rs` |
| failure within ~2 min of its phase | stage tables before and after in the PR (from the journal) |
