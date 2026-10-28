# Tasks

## 1. The pill is 922 pt wide

- [ ] 1.1 Red: change the scenario "The recording pill is as long as ChatGPT's dictation bar" in `app/tests/features/component/visual_feedback.feature` to 922 points; change the widths in `should_be_as_wide_as_its_columns_four_points_apart` (`waveform.rs`) to 210 and 196 columns, 838 and 782 pt (`just test-one`).
- [ ] 1.2 Red: in `level_history.rs`, speech shows after one second of speech and 22 s of silence, and is gone after 20 s of speech and 24 s of silence (`just test-one`).
- [ ] 1.3 Green: `FULL_CAPACITY` 210, `CAPACITY_BESIDE_SIGN` 196 in `app/src/core/domain/level_history.rs`; rename the two `level_history` tests after their new spans.
- [ ] 1.4 `just static`, `just test`; commit `fix: widen the recording pill to the Claude chat input box`.

## 2. Version

- [ ] 2.1 Bump `version` in `app/Cargo.toml` to 0.41.4 (or the next version above `main`, `kit version-check`); `just test`; commit `chore: bump version to 0.41.4`.
- [ ] 2.2 `just check` passes once (`kit verify`).
- [ ] 2.3 Manual check: the user starts a recording in front of the Claude desktop app and confirms the pill matches the chat input box's length.
