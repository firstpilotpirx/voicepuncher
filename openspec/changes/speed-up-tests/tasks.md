# Tasks

## 1. Baseline

- [ ] 1.1 Write `baseline.md` in the change folder: the before table from `just stage-times` (passed runs only) and the five slowest tests of `just test`; commit

## 2. Reconnect test without a real minute

- [ ] 2.1 Red: in `commandspotting_reconnect_it.rs`, make the long-outage test build the client with tenfold shorter timings and an outage past the replay and the tenth attempt; run it and see it fail to compile or fail
- [ ] 2.2 Green: give the command spotting client a timings value (reconnect pauses, replay window) defaulting to today's constants, used by `link.rs`; the app keeps the default; the test passes in < 10 s
- [ ] 2.3 Remove the 3-min override for that test from `.config/nextest.toml`; `just test`; commit

## 3. Give-up test without real deadlines

- [ ] 3.1 Red: in `transcription_openai_it.rs`, assert the hung short recording is dropped after three attempts within 5 s with a 1 s request timeout; run it and see the time bound fail
- [ ] 3.2 Green: set the test's request timeout to 1 s; the test passes in < 5 s; `just test`; commit

## 4. Resilience with short timings

- [ ] 4.1 Red: in `local_services.feature`, reword "A busy service is not restarted" to stream through several checks; in `hangs.rs`, stream for a few check intervals with the scenario's services checked every second; run `just e2e-resilience` and see it fail (the supervisor still checks every 5 s)
- [ ] 4.2 Green: make the supervisor's health-check interval settable by an env var, default 5 s, in the justfile; the `@resilience` steps start their services with 1 s; `just e2e-resilience` passes
- [ ] 4.3 Shorten the confirmation stall in `outages.rs` to 11 s and the dictation past it to match; `just e2e-resilience` passes; `just test`; commit

## 5. Smaller voice control set in check

- [ ] 5.1 Tag `voice_control.feature` scenarios: per command 2–3 synthesized ones plus every `@voice_samples` one stay `@smoke`, the rest get `@regression`
- [ ] 5.2 Add the nextest profile `e2e-regression`, the recipe `e2e-regression` and its stage in `full-check` after `e2e-smoke`, with its time limit; `just e2e-regression` passes
- [ ] 5.3 Update AGENTS.md § Gherkin and § How to verify a change for the `@regression` tag and stage; `just test`; commit

## 6. Duplicates removed

- [ ] 6.1 Compare `commandspotting_websocket_it` with the long-listening and phrase-inside-a-sentence scenarios; remove the side design decision 8 drops and list it in `baseline.md`
- [ ] 6.2 Compare `placed_commands_it` with the after-other-speech, doubled short form, chain and short-breath scenarios; remove the duplicates and list them
- [ ] 6.3 Compare `voice_samples_it` and `voice_forms_it` with the `@voice_samples` scenarios; remove the duplicates and list them
- [ ] 6.4 Compare `chunked_transcription_it` and the `@smoke` normal-pace seam rows with `@seams`; remove the duplicates and list them
- [ ] 6.5 Keep the silence of 1/10/60 s without refinement in one of `file_transcription.feature` and `speech_recognition.feature`; remove it from the other and list it
- [ ] 6.6 Compare the `textrefinement_openai_it` live check with the `@refinement` live scenarios; remove the duplicate and list it
- [ ] 6.7 Update `e2e_smoke_binaries` in the justfile and the nextest groups for removed binaries; `just test`; commit

## 7. Refinement evaluation once

- [ ] 7.1 Red: add a unit test in `refinement_eval.rs` that a shared report is computed once for several callers; see it fail
- [ ] 7.2 Green: the three evaluation scenarios in `evaluation.rs` read one report computed on first use; `just refine-eval` sends the 64 items once; `just test`; commit

## 8. Concurrency measured

- [ ] 8.1 Red: add a unit test in `e2e-test/src/harness_args.rs` that the concurrency comes from the env var and defaults to 4; see it fail
- [ ] 8.2 Green: `cucumber_e2e.rs` takes its concurrency from that function; `just test`
- [ ] 8.3 Run `just e2e-smoke` three times each at 4, 6 and 8; record the times in `baseline.md`; set the fastest green value as the default; commit

## 9. Integration

- [ ] 9.1 Check that `just test` takes < 20 s and `e2e-smoke` in `check` at most half of the baseline; record the after table in `baseline.md`
- [ ] 9.2 Run `just check` once; commit

## 10. Version and manual checks

- [ ] 10.1 No version bump: tests, harness and recipes only (proposal § Release)
- [ ] 10.2 Manual checks by the user: none
