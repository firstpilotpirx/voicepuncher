# Tasks

## 1. Failure notice type and its messages

- [x] 1.1 Red: unit tests in `app/src/core/domain/failure_notice.rs` for the message of every notice, with the remedy for services and accessibility; run, see them fail
- [x] 1.2 Green: add `FailureNotice` and the failing part to `app/src/core/domain/failure_notice.rs`, export from `core/domain`
- [x] 1.3 Add "failure notice" to `docs/ubiquitous-language.md` and extend the `VisualIndicator` entry
- [x] 1.4 `just test` and commit

## 2. Notices for a dictation

- [x] 2.1 Add `show_failure_notice` to `VisualIndicator` (`app/src/core/application/visual_indicator.rs`) with a stub in the macOS overlay that logs the notice; record it in the overlay stand-ins of `app/tests/steps/`
- [x] 2.2 Red: `app/tests/features/component/failure_notices.feature` scenarios for the lost hotkey and voice dictation, the part lost, refinement down, refinement off, a refused paste, a microphone that cannot start, the same failure twice, a new dictation after a notice; steps in `app/tests/steps/failure_notices.rs`; run, see them fail
- [x] 2.3 Green: `refine_or_raw` reports a fallback (`app/src/core/application/text_refiner.rs`), the live worker returns its lost chunk count, the pipeline keeps a pending notice per cycle and shows it instead of the closing hide (`app/src/core/application/pipeline.rs`)
- [x] 2.4 Red: unit test that the macOS paster fails with `AccessibilityMissing` when the process is not trusted; run, see it fail
- [x] 2.5 Green: `AccessibilityMissing` in `core/domain`, checked by the macOS paster and key presser before posting events
- [x] 2.6 `just test` and commit

## 3. Notices for voice control

- [x] 3.1 Red: `failure_notices.feature` scenarios for a refused key press and for command spotting not running at start; run, see them fail
- [x] 3.2 Green: show the notice when a voice command fails and when listening cannot start (`app/src/core/application/pipeline/voice_control.rs`)
- [x] 3.3 `just test` and commit

## 4. The overlay shows the notice

- [x] 4.1 Red: `app/tests/visualfeedback_macos_it.rs` tests that a notice shows its sign in the pill and its message in the text window under it, without activating, hides after its time, restarts its time when shown again, and is not hidden by a stale timer after a recording starts; run, see them fail
- [x] 4.2 Green: notice sign, message in the text window under the pill, and the generation-keyed hide timer in `app/src/adapters/secondary/visualfeedback/macos/`
- [x] 4.3 `just test` and commit

## 5. Version and manual checks

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.19.0
- [x] 5.2 `just check` once and commit
- [x] 5.3 Manual run by the user: with each local service stopped in turn, a hotkey and a voice dictation show a readable notice naming it; the focused app keeps focus; the next dictation starts while a notice is shown
