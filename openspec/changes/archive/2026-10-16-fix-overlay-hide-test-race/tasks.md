# Tasks

## 1. Red: reproduce the early reads

- [x] 1.1 Add a 50 ms `PIPELINE_LAG` (doc comment: it stays so an early read fails every run) before `RecordingIndicator::hide` records `Hidden` in `app/tests/steps/text_insertion.rs`; verify 10 runs of "Paste once the user clicks into a text input on the web page" (`kit run cargo test -p voicepuncher --test cucumber_integration -- --name '<scenario>'`) each fail with `overlay not hidden after Pasted("Meeting moved to Friday."): …`
- [x] 1.2 Add the same lag before `NotingOverlay::show_recording` records `Overlay(Recording)` in `app/tests/steps/processing_stages.rs`; verify 10 runs of "A hotkey dictation shows speech recognition after the stop" each fail with `the stop did not show speech recognition first: …`
- [x] 1.3 Add the same lag before `RecordingSounds::play_listening_on` records `Sound("listening on")` in `app/tests/steps/voice_control/doubles.rs`; verify 10 runs of "Stop dictation without a dictation is ignored" each fail with `a sound played: Sound("listening on"); …`
- [x] 1.4 Run `just test` and verify it fails only in scenarios that use `overlay_hides`, `start_recording` or `listening_on_given`; commit alone as `test: reproduce steps reading the pipeline log too early` with a message stating it is intentionally red (no `just check`: the group is red by design, design D4)

## 2. Green: wait for the last event of each reaction

- [x] 2.1 Make `overlay_hides` wait, within `REACTION`, until `Hidden` follows the paste or clipboard hand-off, keeping its failure message; verify 10 of 10 runs of the 1.1 scenario pass
- [x] 2.2 Make `start_recording` also wait for `Overlay(Recording)` after `RecordingStarted` (`StagesRun::wait_for`); verify 10 of 10 runs of the 1.2 scenario pass
- [x] 2.3 Make `listening_on_given` also wait for `Sound("listening on")` after `ListeningStarted` (`VoiceRun::wait_for`); verify 10 of 10 runs of the 1.3 scenario pass
- [x] 2.4 Verify no bar moved: `git diff origin/main...HEAD -- app/tests/features` is empty, no existing timing constant changes, no `@wip` or retry in the diff, no app code changed
- [x] 2.5 Run `just test` and `just check`, both pass; commit `fix(test): wait for the last event of a pipeline reaction before reading the log`

## 3. Version and manual checks

- [x] 3.1 Verify `version` in `app/Cargo.toml` is unchanged: no bump and no release (proposal § Release)
- [x] 3.2 Manual checks: none (hotkey, recording, overlay, paste and permissions are untouched)
