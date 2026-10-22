# Tasks

## 1. The `e2e-cues` stage (decisions 1, 5)

- [ ] 1.1 Red: `_test-lock-phases` in the `justfile` expects an `exclusive e2e-cues` phase after the light and heavy ones and `e2e-cues` in the `full-check` stage set; run it, it fails.
- [ ] 1.2 Green: `exclusive_stages`, `_lock-phases`, `full_check_model_stages`, the `e2e-cues` and `_e2e-cues` recipes, the `e2e-cues` nextest profile in `.config/nextest.toml`, the `@cues` tag on the cue-timing `Rule:` of `e2e-test/tests/features/voice_control.feature`, its lines in `stage_inputs`; `just test`; commit.

## 2. The exclusive class and the quiet lock (decisions 2, 3)

- [ ] 2.1 Red: `_test-class-locks` expects an exclusive-class run to wait for a shared quiet holder, a new shared quiet run to wait for it, and the exclusive run to keep the light and heavy classes out; run it, it fails.
- [ ] 2.2 Green: `_class-lock exclusive` (machine lock exclusive, then the quiet lock exclusive) and the build stages of `_stages` on the quiet lock shared; `_stages` runs the exclusive phase on the heavy ports; `just test`; commit.

## 3. The load in a late cue's failure (decision 4)

- [ ] 3.1 Red: unit tests in `e2e-test/src/machine_load.rs` for the holders of a scratch lock directory (a dead holder skipped) and the busiest-process rows; run them, they fail.
- [ ] 3.2 Green: `machine_load.rs`; `check_within` and `most_recorded_within` in `e2e-test/tests/steps/voice_control/session/start_cue.rs` append its report; `just test`; commit.

## 4. Measurement and limits (decision 6)

- [ ] 4.1 Three `just e2e-cues` runs beside another worktree's build stages; their times and lock waits into design.md § Measurements; the nextest limits of `e2e-cues` and `e2e-seams` from the measured runs; `just test`; `just check`; commit.

## 5. Docs

- [ ] 5.1 `AGENTS.md` § How to verify a change (Lock classes: the exclusive class and the quiet lock; the stage list of `full-check`; `e2e-seams` and `e2e-cues` recipes) and § Several changes at once; `justfile` comments; commit.
- [ ] 5.2 Version: no bump — checks and the test runner only (proposal § Release); manual checks: none.
