# Tasks

## 1. The waveform drawn with the record sign (decisions 1, 2)

- [ ] 1.1 Red: in `app/tests/visualfeedback_macos_it.rs` add `pump_until(limit, condition)` and make `should_show_the_record_sign_and_waveform_without_a_label` pump until the record sign shows, then read label, waveform and key window at once; run it and see the waveform read `0.0`.
- [ ] 1.2 Green: `LevelWave::elapsed()` in `waveform.rs`; `apply_recording_state` in `pill_states.rs` draws each window's waveform from the feed's wave after drawing the pill; the test passes.
- [ ] 1.3 `just test`; commit.

## 2. Acceptance and version

- [ ] 2.1 Run the test 20 times in a row while another checkout runs `just check` (or under a CPU load started for it); all pass.
- [ ] 2.2 Version: PATCH bump in `app/Cargo.toml` to the next version above `main` (proposal § Release). Manual checks: none — the change touches only the first frame of the overlay's waveform; the user's manual run is deferred by their decision.
