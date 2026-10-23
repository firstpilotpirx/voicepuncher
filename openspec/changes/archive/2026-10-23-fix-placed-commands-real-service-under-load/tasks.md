# Tasks

## 1. A decoding floor for the real-service tests

- [x] 1.1 Red: unit test in `command-spotting/src/confirm/pace.rs` that `Pace::with_floor(floor)` gives `floor` before any confirmation and never passes the deadline; integration test `should_hear_a_clip_a_copy_decodes_past_the_idle_budget` in `command-spotting/tests/it/placed_commands/under_load/mod.rs`: `real_hearing(url)` of a scripted copy decoding for 1.2 s hears the scripted words; run them, see them fail (missing `with_floor`; `real_hearing` built with `UtteranceTranscriber::new` hears nothing)
- [x] 1.2 Green: `Pace::with_floor`, `UtteranceTranscriber::with_decoding_floor(base_url, floor)` (`new` keeps `FLOOR`); `RecentAudio` moves to `confirm/recent_audio.rs` so `command-spotting/src` stays within its directory budget (its `size_baseline.toml` entry dropped); `real_hearing(url)` in `placed_commands.rs` builds the transcriber with `REAL_DEADLINE` as its floor and `engine()` uses it; the new tests pass, `just test` green
- [x] 1.3 `kit ship` the group: `fix(command-spotting): real-service placed-commands tests hear past the idle decoding budget`

## 2. Acceptance

- [x] 2.1 Acceptance bar: with another `just full-check` holding the machine lock, run the real-service tests of `placed_commands.rs` 20 times in a row against the services of a locked run; all 20 pass (2026-10-23: 20/20 passed 12:19–12:49 beside the full-check of sleepy-kapitsa-e5d9b5; a first series of 20 passed 11:27–12:18 beside the full-checks and checks of other branches)
- [x] 2.2 Version: unchanged (`app/Cargo.toml` stays at 0.37.4, no release); manual checks: none
