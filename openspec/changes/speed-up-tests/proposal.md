## Why

Checks take about 45 % of a change's time and the machine-lock wait another 35 %; the user counts both as test time. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7, 140 recorded runs of `just check` / `just full-check`, 14–18 Oct, ≈ 950 min) measured where it goes:

- `e2e-smoke` is the biggest stage: median 269 s in `check`, 343 s in `full-check`, 352 min over the 140 runs. Inside it the 178 `@smoke` Gherkin scenarios take 250–270 s; 127 of them are voice control, run one at a time, streaming audio in real time (100 ms frames, 15 s pauses, reactions awaited up to 30 s).
- `just test` takes 65 s per run, and runs after every edit; the ~680 tests without models fit in about 15 s, but one test, `commandspotting_reconnect_it::should_be_back_within_seconds_once_the_service_answers_after_a_long_outage`, waits a real 60 s outage (`LONG_OUTAGE`), so the whole stage waits for it.
- `transcription_openai_it::should_give_up_a_short_recording_the_service_never_answers_within_half_a_minute` waits about 23 s of real request deadlines (3 attempts against a hanging stub) inside `e2e-smoke`.
- `e2e-resilience` takes 176 s for 8 scenarios: each starts its own services; outages include a 15 s stall and a minute of streaming.
- Duplicates: the same behavior is tested twice in several places (below).
- The text refinement model is not the bottleneck: `refine-eval` takes about 25 s a run (3 % of the time); only 6 scenarios use speech recognition and the LLM together.

## What Changes

- **Smaller voice control set in `check`:** for each voice command, `check` runs 2–3 scenarios with synthesized speech (`say`) and the scenarios on the user's own recordings (`~/.voicepuncher/voice-samples/`); the full voice control set runs in `full-check`.
- **Voice control scenarios run in parallel:** the voice control scenarios (mostly waiting on real-time audio, not on the GPU) run several at a time instead of one by one, as the speech recognition service already runs several copies in parallel; the gain is measured.
- **No real waiting for timeouts in tests:** tests that check a timeout, a give-up or an outage get short timings for the test (injected deadlines, retry pauses, outage lengths) instead of waiting the production length in real time; what they prove stays the same (e.g. the client keeps trying through an outage of a minute and is back within seconds; it gives up after 3 attempts). Covers at least the 60 s outage test, the 23 s give-up test and the 15 s stall and minute-long streaming of `@resilience`.
- **Duplicates removed**, keeping one test per behavior at the right level:
  - `commandspotting_websocket_it` (long listening, command phrase inside a sentence) vs `voice_control.feature` long listening and phrase inside a sentence;
  - `placed_commands_it` (after other speech, doubled short form, chain, short breath) vs the same `voice_control.feature` scenarios;
  - `voice_samples_it` / `voice_forms_it` vs the 22 `@voice_samples` scenarios on the same recordings;
  - `chunked_transcription_it` vs the `@seams` scenarios (same conditions and lead-ins), and the `@smoke` normal-pace seam rows vs `@seams` normal-pace rows;
  - silence of 1/10/60 s without refinement in both `file_transcription.feature` and `speech_recognition.feature`;
  - `textrefinement_openai_it` live check vs the `@refinement` live dictation scenarios;
  - the refinement evaluation runs its 64-item set three times; once is enough.

Current stage: PoC. Target stage: PoC.
Scope: test code, Gherkin features and steps, the cucumber harness, `.config/nextest.toml` limits, `justfile` stage selection for `check` and `full-check`; app or command spotting code only to make a timing injectable for tests.
Acceptance bar: every behavior the removed or shortened tests proved is still proved by a remaining test (traced in the PR); `just test` under 20 s; `e2e-smoke` in `check` at least halved against the 269 s median; `just check` and `just full-check` pass; before/after stage tables in the PR.
Out of scope: fixing flaky tests (`fix-flaky-tests`); smaller accuracy samples; skipping `e2e-seams` / `e2e-resilience` when their paths are untouched; any product behavior.

## Capabilities

### Modified Capabilities

- `voice-control`: which scenarios run in `check` and `full-check`; scenarios run in parallel; duplicate tests removed.
- `speech-recognition`: duplicate seam tests removed; timeouts tested with short timings.
- `text-refinement`: evaluation set run once; duplicate live check removed.
- `local-services`: outage and stall scenarios use short timings.

## Impact

- `app/tests/commandspotting_reconnect_it.rs`, `app/tests/transcription_openai_it.rs`, `app/tests/commandspotting_websocket_it.rs`, `app/tests/voice_samples_it.rs`, `app/tests/voice_forms_it.rs`, `app/tests/chunked_transcription_it.rs`, `app/tests/textrefinement_openai_it.rs`, `command-spotting/tests/placed_commands_it.rs`.
- `e2e-test/tests/features/*.feature`, `e2e-test/tests/steps/`, `e2e-test/tests/cucumber_e2e.rs` (parallel runs), `.config/nextest.toml`, `justfile` (`e2e_smoke_binaries`, `_e2e-smoke`, `check`, `full-check`), AGENTS.md § How to verify a change (stage times).

## Open points

- How the smaller voice control set is chosen: proposed — a tag (e.g. `@smoke` for the per-command set, a new tag for the rest) that `check` and `full-check` select.
- Whether `full-check` runs the full voice control set always or only when voice control paths change: proposed — always for now.
- How many scenarios run at once and how shared services hold the load (three copies of the command speech recognition service, one command spotting service): proposed — measure 2, 3 and 4 and pick the fastest that stays green.
- The 60 s outage test (user's idea, 2026-10-18): nextest already runs it beside the other tests, but `just test` cannot end before its longest test, so it alone sets the 65 s. Two ways out: (a) move it out of `just test` into a stage that runs anyway beside long work (e.g. started at once with `e2e-smoke`, whose Gherkin scenarios take 250–270 s), so its 60 s hides behind them; (b) give it a short outage for the test. Proposed — (b) when the client's retry timings can be injected without changing what the spec promises (a minute-long outage survived); else (a). Either way only this one test needs the long outage, no other test waits for it.
- Which side of each duplicate stays: proposed — the Gherkin scenario where a spec scenario traces to it, the plain Rust test where it checks an adapter detail.
- Target version: proposed no release — only tests and recipes change; PATCH if a timing becomes configurable in app code.

## Release

SemVer: no release — tests, harness and recipes only; no behavior change.
Target version: 0.18.0.
