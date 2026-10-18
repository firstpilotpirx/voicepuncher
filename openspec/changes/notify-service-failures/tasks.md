# Tasks

## 1. Failure notice type and its messages

- [ ] 1.1 Red: unit tests in `app/src/core/domain/failure_notice.rs` for the message of every notice, with the remedy for services and accessibility; run, see them fail
- [ ] 1.2 Green: add `FailureNotice` and the failing part to `app/src/core/domain/failure_notice.rs`, export from `core/domain`
- [ ] 1.3 Add "failure notice" to `docs/ubiquitous-language.md` and extend the `VisualIndicator` entry
- [ ] 1.4 `just test` and commit

## 2. Notices for a dictation

- [ ] 2.1 Add `show_failure_notice` to `VisualIndicator` (`app/src/core/application/visual_indicator.rs`) with a stub in the macOS overlay that logs the notice; record it in the overlay stand-ins of `app/tests/steps/`
- [ ] 2.2 Red: `app/tests/features/component/failure_notices.feature` scenarios for the lost hotkey and voice dictation, the part lost, refinement down, refinement off, a refused paste, a microphone that cannot start, the same failure twice, a new dictation after a notice; steps in `app/tests/steps/failure_notices.rs`; run, see them fail
- [ ] 2.3 Green: `refine_or_raw` reports a fallback (`app/src/core/application/text_refiner.rs`), the live worker returns its lost chunk count, the pipeline keeps a pending notice per cycle and shows it instead of the closing hide (`app/src/core/application/pipeline.rs`)
- [ ] 2.4 Red: unit test that the macOS paster fails with `AccessibilityMissing` when the process is not trusted; run, see it fail
- [ ] 2.5 Green: `AccessibilityMissing` in `core/domain`, checked by the macOS paster and key presser before posting events
- [ ] 2.6 `just test` and commit

## 3. Notices for voice control

- [ ] 3.1 Red: `failure_notices.feature` scenarios for a refused key press and for command spotting not running at start; run, see them fail
- [ ] 3.2 Green: show the notice when a voice command fails and when listening cannot start (`app/src/core/application/pipeline/voice_control.rs`)
- [ ] 3.3 `just test` and commit

## 4. The overlay shows the notice

- [ ] 4.1 Red: `app/tests/visualfeedback_macos_it.rs` tests that a notice shows its message in a wider non-activating window, hides after its time, restarts its time when shown again, and is not hidden by a stale timer after a recording starts; run, see them fail
- [ ] 4.2 Green: notice state, window sizing and the generation-keyed hide timer in `app/src/adapters/secondary/visualfeedback/macos/`
- [ ] 4.3 `just test` and commit

## 5. Version and manual checks

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to 0.16.0
- [ ] 5.2 `just check` once and commit
- [ ] 5.3 Manual run by the user: with each local service stopped in turn, a hotkey and a voice dictation show a readable notice naming it; the focused app keeps focus; the next dictation starts while a notice is shown
