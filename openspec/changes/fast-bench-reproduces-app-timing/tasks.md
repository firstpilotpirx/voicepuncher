# Tasks

## 1. Replayed delivery in the decoder

- [x] 1.1 Red: session unit test `should_see_a_decode_when_it_would_have_ended` in `command-spotting/src/streaming/session/tests/paces.rs` (a scripted recognizer taking 30 ms per decode, replayed delivery); `just test-one` fails
- [x] 1.2 Green: `Delivery` (as decoded, replayed) in `decoder.rs`: jobs stamped with the stream time fed, updates with their visible time, `arrived(now)` releasing only the visible ones after the thread caught up, `caught_up` releasing all; `Utterance::feed` and `silence` pass `now`; `SessionCfg.delivery`; `StreamingEngine::session_replaying`; `just test-one` passes
- [x] 1.3 Red: session unit test `should_keep_delivering_as_decoded_by_default` in `tests/paces.rs` (the default delivery reports a scripted decode in the frame it arrived); `just test-one` fails, then passes
- [x] 1.4 `just static`, `just test`, commit

## 2. The fast pace's delay and footer

- [x] 2.1 Red: bench unit tests `should_count_the_fast_delay_from_the_stream_position` and `should_name_what_the_fast_pace_does_not_replay` in `command-spotting/src/bench/`; `just test-one` fails
- [x] 2.2 Green: `run_clip` opens a replaying session in the fast pace, stamps each report with the stream position and counts its delay from the clip's word end; `not_replayed()` text in `bench/summary.rs`; `streaming_bench.rs` prints it after the summaries in `--fast` and its `--fast` help says the delays are replayed; `just test-one` passes
- [x] 2.3 `just static`, `just test`, commit

## 3. The paces agree

- [ ] 3.1 Red: `Rule: The fast streaming bench reproduces the service's timing` in `e2e-test/tests/features/voice_control.feature` with the scenarios "A lone restart gives the same delay in both paces" (`@regression`) and "The user's recordings give the same delays in both paces" (`@voice_samples @regression`), steps in `e2e-test/tests/steps/voice_control/streaming/paces.rs` (the lone-restart clip built from synthesized speech, a sample of 3 recorded clips with a command); the E2E recipe runs them red
- [ ] 3.2 Green: the steps run both paces through `run_clip` and compare each delay within 0.1 s; `just e2e-regression` on the new rule passes
- [ ] 3.3 Measure by hand: `just streaming-bench --recorded --word start` and `--word stop` in real time and `--fast`, the lone restart in both paces; write median, p90 and the largest per-clip difference into `design.md` § Measurements, and the margin in the spec when the measurement contradicts 0.1 s
- [ ] 3.4 `docs/ubiquitous-language.md` § Streaming bench: the fast pace replays the service's clock; `just static`, `just test`, `just check`, commit

## 4. Version and manual checks

- [ ] 4.1 `version` in `app/Cargo.toml` stays 0.46.2 (no release); manual checks: none
