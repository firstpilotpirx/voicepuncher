# Proposal

## Why

Agents run `just test` after every edit and `just check` after every task group, so the checks set the pace of every change — and they have grown far past the times the docs promise. Measured on 2026-10-14 (`research/README.md`): `just test` 113 s (docs: ~1 min), `just check` ≈ 22 min (docs: ~2 min; it was 8 min two days ago), `just full-check` ≈ 28 min (docs: ~10 min). A change with four task groups now spends about two hours in checks.

Most of that time is waiting, not testing:

| Cause | Where | Cost per `check` |
|---|---|---|
| A fixed wait for "no more events" after audio is streamed to the command spotting service: 4 s per pass, 1.5 s per utterance — although the service answers every frame in order and reads no clock | `voice_forms_it` 397 s, `voice_samples_it` 76 s, 75 voice-control E2E scenarios, "Ordinary speech triggers no command" | ≈ 12 min |
| The E2E harness asks for 4 concurrent scenarios, but the steps block and cucumber polls them all on one thread: one at a time takes 535 s, "four at a time" 542 s | `cucumber_e2e` | the overlap it was meant to give |
| macOS `say` costs ≈ 1 s per call, even for "press enter", and slows down when called in parallel; the same phrases are synthesized on every run | `engine_it` 43 s, E2E harness unit tests 21 s, `chunk_cuts_it` 18 s, ≈ 1 s in every voice E2E scenario | ≈ 2.5 min |
| `just test` runs its ≈ 30 test binaries one after another | `_test` | 113 s; run at once 70 s, before any other saving |
| The E2E test harnesses are built a second time in the release profile, although the app under test is the release binary run as a process and the test-side code gains nothing from optimization (`engine_it` 43 → 41 s) | `_e2e-smoke` | 36 s after every edit |
| Every E2E recipe starts and stops the services it did not find running | `check` and `full-check` without running services | 11 s per start, 3–4 starts |

A slow check also fails more: the bench scenario "Ordinary speech triggers no command" failed once with `os error 35`, because the test streams audio faster than the busy service reads it and the client's 2 s write timeout fires.

## What Changes

- **Wait for the service, not for the clock.** The app's command spotting client can wait until the service has answered all audio sent so far (a WebSocket ping; the service already answers it after the events for every earlier frame). The voice test helpers, `voice_forms_it` and `voice_samples_it` wait this way instead of a fixed quiet time; while streaming they wait every few seconds of audio, so they never send faster than the service reads (fixes the `os error 35` flake).
- **Synthesize each phrase once.** The test helpers that call `say` (E2E harness, command spotting tests) keep every synthesized phrase in a cache on the machine, keyed by voice, rate, audio format, text and macOS build; only a phrase not in the cache runs `say`.
- **E2E scenarios really run four at a time.** The blocking work of the E2E steps runs off the cucumber thread, so `MAX_CONCURRENT_SCENARIOS` takes effect (the accuracy benchmarks, which load the services on purpose, run alone); independent recordings in `voice_forms_it` run a few at a time as well.
- **`just test` runs its test binaries at once.** One build, then every binary in parallel; each binary's output is printed whole when it finishes, failures and the exit code as today, plus the slowest binaries with their times.
- **E2E test code in the dev profile.** The E2E recipes run their test binaries from the dev build `just test` already made; the app binary they start stays the release build.
- **Services once per check.** `check` and `full-check` start the three services (speech recognition, text refinement, command spotting) they did not find running once, before their E2E stages, and stop them at the end; the E2E recipes run alone keep their own start and stop.
- **Times in sight.** `check` and `full-check` end with the time of each stage; the times in the `justfile`, `AGENTS.md` and `openspec/config.yaml` become the measured ones.

Every test that runs today still runs, with the same bars (WER < 30 %, seam rules, every voice form and recording).

## Capabilities

### New Capabilities

None. This is test and check tooling, not app behavior: `skip_specs: true`.

### Modified Capabilities

None.

## Impact

- `app/src/adapters/secondary/commandspotting/websocket/client.rs`: a method that waits until the service has answered all audio sent so far. The app does not call it; its own path is unchanged.
- `e2e-test/src/voice_commands.rs`, `e2e-test/src/speech.rs` (speech cache, waiting for the service), `e2e-test/tests/steps/` (blocking work off the cucumber thread), `app/tests/voice_forms_it.rs`, `app/tests/voice_samples_it.rs`, `command-spotting/tests/` (speech cache).
- `justfile`: `_test`, `_check`, `_full-check`, the E2E recipes, recipe comments. `AGENTS.md` § How to verify a change and `openspec/config.yaml` `context`: measured times.
- A cache directory of synthesized speech under `~/.cache/voicepuncher/` (a few MB), shared by every checkout.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** timed again under the setup of `research/README.md`: `just test` ≤ 45 s, `just check` ≤ 6 min, `just full-check` ≤ 13 min (its `cargo clean` and cold compile included); `just check` passes, `just full-check` passes bar the known seam failures that `fix-dropped-phrase-at-word-pauses` owns; the bench scenario "Ordinary speech triggers no command" passes three runs in a row; the user's manual run confirms that voice commands still work.
- **Out of scope:**
  - new tools (`cargo-nextest`, `sccache`) — not in the tech matrix;
  - dropping `cargo clean` from `full-check` — the user keeps it;
  - fixing the seam failures (`fix-dropped-phrase-at-word-pauses`);
  - moving tests between `check` and `full-check`, dropping a test, or loosening a bar;
  - scenarios that are real time by design: the hands-free sessions, the keep-alive and hang tests of the command spotting client, the `@serial` component scenarios;
  - merging integration test binaries; build profile tuning; faster models or service settings;
  - kit changes (for example skipping stages of `full-check` that `check` already passed on the same commit).

## Release

No release: no app behavior changes; `app/Cargo.toml` keeps 0.10.0.
