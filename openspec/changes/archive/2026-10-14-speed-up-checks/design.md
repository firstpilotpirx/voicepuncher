# Design

## Context

The timings and their causes are in `research/README.md`; the motivation is in proposal.md § Why. The facts that shape the approach:

- **The command spotting service is synchronous per connection.** `command-spotting/src/server.rs` reads one message at a time; for a binary frame it runs `Session::accept`, which confirms an utterance with the speech recognition service inside the call, and sends every resulting event before it reads the next message. Nothing in `command-spotting/src` reads a clock. tungstenite queues the pong for a ping and flushes it at the start of the next `read()`, so the pong leaves after every event for the frames before the ping.
- **The test-side streaming helpers guess when the service is done.** `e2e-test/src/voice_commands.rs` (`spotting_events`, `carried_out_by_app`) waits until 4 s pass without an event; `voice_samples_it` waits 1.5 s after each utterance. They send audio as fast as they can, so the service lags behind: `carried_out_by_app` switches the phrase set when an event arrives, which is later in the audio than where the app would switch it, and a large backlog fills the socket (`os error 35`).
- **`SpottingClient`** (`app/src/adapters/secondary/commandspotting/websocket/client.rs`) already sends pings for its keep-alive and ignores pongs in `read_events`.
- **cucumber-rs polls every scenario on the thread of `futures::executor::block_on`.** Synchronous steps block that thread, so `max_concurrent_scenarios(4)` overlaps nothing.
- **`say`** starts a speech synthesis process per call (≈ 1 s, even for two words), and calls in parallel slow each other down. The E2E harness calls it in `speech.rs` (`say`) and `voice_commands.rs` (`spoken`); the command spotting service's tests have their own copy in `engine_it.rs` and `placed_commands_it.rs`.
- **The E2E recipes compile their test binaries with `--release`** (36 s after an edit). Their app under test is `target/release/voicepuncher`, started as a process (`sut_bin`), which `_build` makes; the test-side code is the same with or without optimization (`engine_it` 43 → 41 s, `chunk_cuts_it` 18 → 16 s).
- **Each E2E recipe starts the services it does not find running and stops them afterwards**, with a marker file per service. A second recipe in the same `check` repeats both.

## Goals / Non-Goals

**Goals:**
- Every voice test waits exactly as long as the service needs, and never sends faster than the service reads.
- A phrase is synthesized once per machine, not once per run.
- E2E scenarios overlap up to the harness's limit of 4.
- `just test` takes about as long as its slowest test binary.
- No test is dropped, no bar or tolerance is changed, no scenario text changes.

**Non-Goals:**
- Changing the command spotting service or its protocol.
- Making real-time scenarios (hands-free sessions, keep-alive, hang, `@serial` component scenarios) faster.
- A general-purpose test runner.

## Decisions

### 1. "Answered so far" is a ping round trip

`SpottingClient` gets a method that sends a ping with a payload of its own, reads until the pong with that payload, and returns every event read on the way; it fails when the service sends nothing for the client's silence limit (10 s), as `keep_alive` does. The service needs no change: the ordering guarantee is in its loop (Context).

The helpers use it like this:
- `spotting_events` (fixed phrase set): every 2 s of audio and at the end. The periodic wait keeps at most 2 s of audio (128 kB) in flight, well under the loopback socket buffer, so a write never blocks into the 2 s write timeout.
- `carried_out_by_app` (the phrase set switches after a command): after every 100 ms frame, so a switch takes effect at the next frame — where the app, with a service that keeps up in real time, switches it too.
- `voice_samples_it`: at the end of each utterance (and every 2 s of audio), before it switches the phrase set for the next one.
- `voice_forms_it`: through `carried_out_by_app`.

*Alternatives:* a "flush" message in the protocol (needs a service change for what a ping already gives); a shorter quiet time (still a guess, still flaky under load); a test-only WebSocket client in `e2e-test` (the voice E2E tests would no longer exercise the app's real client).

### 2. Speech synthesis cache

One function synthesizes 16 kHz mono 16-bit speech for (voice or the default voice, words per minute or the default rate, text). It looks the result up in `~/.cache/voicepuncher/speech/<key>.wav` (directory overridable by `VOICEPUNCHER_SPEECH_CACHE`) and runs `say` only on a miss, writing to a temporary file in the cache directory and renaming it into place, so parallel tests never read half a file. The key is a stable 64-bit FNV-1a hash of the macOS build (`sw_vers -buildVersion`, read once per process), voice, rate, audio format and text — a macOS update that changes the voices gives new entries; a stale entry still holds valid speech of the same words.

`e2e-test/src/speech.rs` and `voice_commands.rs` call it; the command spotting tests get a copy in `command-spotting/tests/common/mod.rs` with the same key, so both share entries. The copy is ≈ 90 lines of test code; sharing it would make the command spotting service's tests depend on the app crate through `voicepuncher-e2e-test`, or add a workspace crate for one function.

*Alternatives:* committing WAV fixtures (binary churn, and regenerating them is the same work); caching in `target/` (lost per worktree and on every `full-check`'s `cargo clean`).

### 3. Blocking E2E work off the cucumber thread

The harness gets `off_thread(f)`: it runs `f` on a new thread and returns a future that resolves to its result (a `futures` oneshot channel; no new dependency). The slow `When` steps become `async` and move their blocking call (streaming to a service, running the app, a hands-free session, a transcription) into `off_thread`, then store the result in the world. Assertions stay in `Then` steps. `MAX_CONCURRENT_SCENARIOS` stays 4: the reason in its comment (the speech recognition service resets connections beyond that) still holds. Scenarios tagged `@accuracy` run alone (cucumber's `which_scenario`, next to the existing `@serial`): they load the services on purpose — the corpus sample is transcribed two at a time — and once scenarios really overlapped, the voice-control accuracy scenario's confirmations queued behind the corpus transcriptions until the client's 10 s silence limit (2 of 3 bench runs failed). The hands-free sessions run alone too (`@serial` on their rule in `voice_control.feature`): they run in real time and wait for the speech recognition service, and with other scenarios' transcriptions queued ahead of them a command was missed. Hearing the two listening passes of a scenario at once was tried and dropped: the speech recognition service then stalled 10 s on a confirmation and the bench failed.

`voice_forms_it` maps its forms through `map_concurrently` (`e2e-test/src/concurrency.rs`) four at a time; `voice_samples_it` keeps one connection per session in order (a session's phrase set depends on the utterance before), and runs its sessions four at a time.

*Alternatives:* a multi-threaded async runtime (cucumber still polls the scenarios inside one future, so blocking steps would still serialize); more than 4 at a time (the services handle requests one at a time — speech recognition in particular — so more clients only queue).

### 4. `just test` runs its binaries at once

`_test` builds the three selections it runs today with `cargo test --no-run` (app tests, command spotting crate, E2E harness unit tests), takes the test executables from cargo's `Executable` lines (no `jq`, which older macOS lacks), and starts all of them at once, each in its package directory, with the environment the `justfile` exports. Each binary's output goes to its own file and is printed whole, under cargo's usual `Running <source> (<executable>)` line, when the binary ends, so the failure blocks that `kit run` parses stay intact. At the end it prints the five slowest binaries with their times and exits non-zero naming the binaries that failed. Doc tests are dropped: the three selections have none.

*Alternatives:* `cargo-nextest` (not in the tech matrix, and it cannot list the cucumber harnesses, which have no libtest CLI); `just`'s `[parallel]` over three `cargo test` calls (the app's ≈ 25 binaries would still run one after another, and the outputs interleave line by line); merging test binaries (a layout change `AGENTS.md` does not ask for).

### 5. E2E test binaries in the dev profile

The E2E recipes drop `--release` from their `cargo test` calls; the app tests among them reuse what `just test` already built, the E2E harness is built once in dev. They keep depending on `_build`, which makes the release app binary that `sut_bin` starts, and `start-model-servers` keeps building the command spotting service in release. The only E2E code that runs app code in-process in real time is the hands-free session (`voice_control/session.rs`); its reaction limit is 30 s against reactions of a few hundred ms. If it turns out slower or flaky in dev, `cucumber_e2e` alone goes back to `--release`.

### 6. Services once per `check` and `full-check`

`_check` and `_full-check` become bash recipes that run their stages one by one through a timing function, start the services they did not find running (the existing `_start-model-servers-for-e2e`) right before the first E2E stage, and stop them in an `EXIT` trap. They export `VOICEPUNCHER_E2E_SERVICES_HELD=1`; the E2E recipes' own start and stop do nothing while it is set, so an inner recipe never stops a service the outer one started (the marker files are per service, not per run). `full-check` keeps `cargo clean` first.

### 7. Stage times

The timing function prints, after the last stage or the one that failed, one line per stage with its time and a total, in the style of `~/.claude/rules-custom/terminal-output.md`. `_test` prints its per-binary times (decision 4). The time estimates in recipe comments, `AGENTS.md` § How to verify a change and `openspec/config.yaml` are replaced by the times measured at the end of this change.

## Test levels

No spec scenario changes (`skip_specs: true`). The new code is tested as follows:

| What | Level |
|---|---|
| `SpottingClient` returns every event for the audio sent before the wait, and only after the service answered it | adapter integration (`app/tests/commandspotting_client_it.rs`, new because `commandspotting_websocket_it.rs` is over the file budget; stub service that answers each frame late) |
| `SpottingClient`'s wait fails when the service stops answering, not before its 10 s silence limit | adapter integration (same file, a hanging stub service) |
| The speech cache synthesizes a phrase once and keeps voice, rate and text apart | unit (`e2e-test`, `#[cfg(test)]`, synthesizer injected) |
| The voice tests still pass with the new waits | the tests themselves: `voice_forms_it`, `voice_samples_it`, `commandspotting_websocket_it`, `@voice_control` E2E scenarios in `just e2e-smoke`, `e2e-quick`, `e2e-bench` |
| `just test` reports a failing binary and exits non-zero | the Red run of the speech cache (task 2.1) |
| The `os error 35` flake is gone | three `just e2e-bench` runs in a row (task 6.2); no stub service reproduces it (task 1.4), so the bug-fix rule's committed red test is waived |
| Times | `just test`, `just check`, `just full-check` timed as in `research/README.md` |
| Voice commands in the real app | manual run by the user (the command spotting client changed) |

## Risks / Trade-offs

- [Switching the phrase set one frame after the command, instead of after the backlog, changes what `carried_out_by_app` hears] → It is closer to the app; a scenario or recorded form that changes its result shows a real difference between the test and the app and is reported, not tuned away.
- [Real overlap of E2E scenarios loads the services as never before; the hands-free sessions run against reaction limits] → 4 at a time, as the harness already intended; the limits are 15–30 s against sub-second reactions; a scenario that becomes flaky is reported with its timing.
- [A stale cache entry after a voice changes without a macOS update] → the entry is still speech of the same words; deleting `~/.cache/voicepuncher/speech` resynthesizes everything.
- [Parallel test binaries load the CPU together; `@serial` component scenarios have real-time waits] → measured: all 30 binaries at once pass (`research/README.md`); with the cache the three `say`-heavy binaries no longer compete.
- [A hand-written runner in `_test` replaces cargo's] → it prints cargo's per-binary output unchanged and keeps the exit code; `kit run` parses it as before.
