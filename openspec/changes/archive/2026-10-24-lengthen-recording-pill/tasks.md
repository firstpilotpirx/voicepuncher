## 1. A wider pill

- [x] 1.1 Red: in `app/tests/features/component/visual_feedback.feature`, under `Rule: The recording shows a pulsing record sign and a slow scrolling level waveform`, the scenario "The recording pill is as long as ChatGPT's dictation bar": a recording starts, the pill's glass frame is 727 pt wide
- [x] 1.2 Green: `FULL_CAPACITY` 160 and `CAPACITY_BESIDE_SIGN` 148 in `level_history.rs`, their comments (the last 16 s)
- [x] 1.3 Version: bump `app/Cargo.toml` to 0.38.5 (PATCH)
- [x] 1.4 `just test`; commit
