# Tasks

## 1. Reproduce the missed quiet clear (Red)

- [x] 1.1 In `command-spotting/tests/engine_it.rs` add an integration test: each of the six phrases and the six short forms, scaled so its loudest 10 ms has an RMS of 0.005, gives exactly one speech segment; it fails at "clear all text" with none; commit as `test: reproduce the quiet "clear all text" getting no speech segment` (intentionally red)

## 2. The detector hears the stream raised

- [x] 2.1 Red: unit tests in `command-spotting/src/engine.rs` for `raised` (doubles a window, clamps at ±1.0) and for `phrase_span` keeping a first token 0.32 s before the next (design D3); `just test` fails
- [x] 2.2 Green: `DETECTOR_GAIN` 1.5 and `raised` feeding the detector in `detect_speech`, `save_speech` writing the level heard, `MAX_TOKEN_GAP_SECS` 0.4 (design D1–D3); add an integration test in `engine_it.rs` that 20 s of room noise at RMS 0.003, 0.01 and 0.03 gives no segment; `just test` passes, `should_spot_quiet_speech` included; `kit verify after-group`, commit

- [x] 2.3 Red: in `engine_it.rs`, the user's two recorded "start dictation"s 0.32 s apart (session-1, a click between them) give two segments; fails at × 2 with one; commit (intentionally red)
- [x] 2.4 Green: `DETECTOR_GAIN` 1.5 (design D1); `just test` passes; commit

## 3. E2E

- [x] 3.1 Remove `@wip` and its comment from the "clear all text" Examples of "A command said in a quiet voice is recognized" in `e2e-test/tests/features/voice_control.feature`, merging the row into the main Examples; the scenario passes with the branch's service, and the quiet "start" and "let's start" rows still pass (design D4); `kit verify after-group`, commit

## 4. Version and checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to the next patch above `main`; `just test` passes
- [x] 4.2 `just check` passes
- [x] 4.3 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions do not change)
