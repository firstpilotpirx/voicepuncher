## Context

`should_show_the_record_sign_and_waveform_without_a_label` (`app/tests/visualfeedback_macos_it.rs`) sometimes reads a tallest waveform column of `0.0` where it expects a dot of `2.0`, on branches that did not touch the overlay, while other checkouts loaded the machine.

The cause is in the overlay, not only in the test. `apply_recording_state` (`app/src/adapters/secondary/visualfeedback/macos/pill_states.rs`) draws the record sign on the main queue but leaves the waveform's column layers untouched: their frames are zero until `schedule_waveform_tick` fires its first tick 16 ms later and calls `WaveformView::draw`. The test waits a fixed 0.2 s (`DRAWN_SECS`) after `show_recording`; when the machine is busy, the main-queue block that draws the state runs late in that window and the first tick has not run when the test reads the columns. The record sign is there, the waveform is an empty strip — the same empty strip the user sees for the first frame of every recording.

## Goals / Non-Goals

**Goals:**
- The recording state draws its waveform in the same main-queue block as its record sign, so the pill is never seen with an empty waveform.
- The test reads the overlay once the recording state is drawn, not after a guessed time, and fails if the waveform is missing at that moment.

**Non-Goals:**
- Reworking the other fixed `DRAWN_SECS` waits of the harness; they read state that is drawn in one block and do not flake.
- Changing the waveform's pace, look or the tick period.

## Decisions

1. **Draw the waveform in `apply_recording_state`.** After drawing each window's pill, lock the feed's `LevelWave` and call `ws.pill.waveform.draw(&wave, wave.elapsed(), columns, sliding)` — without recording a level, so the history is unchanged and the first tick goes on as before. `LevelWave` gets `elapsed()` (the time since its origin). A recording just started has a history of silence, so it draws dots; a recording redrawn when the commands-unavailable sign toggles draws the history it has. Alternative — record a level at the start: rejected, it adds a column the tick did not take.
2. **The test pumps until the record sign shows.** A helper `pump_until(limit, condition)` runs the main run loop in 1 ms steps until `condition` holds or `limit` (2 s) passes. The test waits for `first_shows_recording()`, then reads the label, the waveform and the key window at once. Because the first tick fires at least 16 ms after the state is drawn, a 1 ms step reads the waveform before any tick: without decision 1 the test fails every run (Red), with it the waveform is a dot already (Green). Alternative — a longer fixed wait: rejected, it only makes the flake rarer and hides the empty first frame.

## Risks / Trade-offs

- [A 1 ms pump step overruns 16 ms on a starved machine, so the Red run could pass once] → only the Red step depends on it; it is run and seen failing before the fix.

## Test levels

- Integration (`visualfeedback_macos_it.rs`, plain Rust per AGENTS.md § Gherkin): the changed test proves the new scenario "The waveform shows together with the record sign" and keeps proving "A new recording starts with a dotted waveform".
- Unchanged scenarios keep their existing integration tests in `visualfeedback_macos_it.rs` and gain none: "Speech leaves tall columns that slide to the left", "Silence leaves a dotted line", "The waveform keeps the last six seconds" and "The waveform moves at half its former pace" (the level history's unit tests and `should_raise_waveform_columns_for_a_loud_level`), "The record dot pulses in silence too" (`should_pulse_the_record_dot_in_silence`).
- No Gherkin change: the overlay has no slice or component scenario for the waveform.

## Acceptance

The test passes in 20 consecutive runs while another checkout runs `just check` (proposal acceptance bar); when no other check runs at that time, 20 runs under a CPU load started for the purpose stand in.
