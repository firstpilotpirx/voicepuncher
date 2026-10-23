# Tasks

## 1. A decoding floor for the real-service tests

- [ ] 1.1 Red: unit test in `command-spotting/src/confirm/pace.rs` that `Pace::with_floor(floor)` gives `floor` before any confirmation and never passes the deadline; integration test `should_hear_a_clip_a_copy_decodes_past_the_idle_budget` in `command-spotting/tests/it/placed_commands/under_load/mod.rs`: `real_hearing(url)` of a scripted copy decoding for 1.2 s hears the scripted words; run them, see them fail (missing `with_floor`; `real_hearing` built with `UtteranceTranscriber::new` hears nothing)
- [ ] 1.2 Green: `Pace::with_floor`, `UtteranceTranscriber::with_decoding_floor(base_url, floor)` (`new` keeps `FLOOR`); `real_hearing(url)` in `placed_commands.rs` builds the transcriber with `REAL_DEADLINE` as its floor and `engine()` uses it; the new tests pass, `just test` green
- [ ] 1.3 `kit ship` the group: `fix(command-spotting): real-service placed-commands tests hear past the idle decoding budget`

## 2. Acceptance

- [ ] 2.1 Acceptance bar: with another `just full-check` holding the machine lock, run the real-service tests of `placed_commands.rs` 20 times in a row against the services of a locked run; all 20 pass
- [ ] 2.2 Version: unchanged (`app/Cargo.toml` stays at 0.37.4, no release); manual checks: none
