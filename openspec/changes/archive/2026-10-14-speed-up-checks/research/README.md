# Baseline timing of the checks

Measured on 2026-10-14 at `main` 51c1c56 (0.10.0) in a fresh worktree, under the machine-wide test lock:
Apple Silicon, 10 cores (8 performance + 2 efficiency), 64 GB; whisper `ggml-small`, Qwen2.5-1.5B Q4_K_M, the
keyword spotting model `sherpa-onnx-kws-zipformer-gigaspeech-3.3M`; the three services already running (the user's),
so no service start is counted unless stated. Every output line was timestamped; a step's time is wall time.

## `just test` — 113 s warm, 123 s after an edit to `app/src`, ≈ 160 s cold

| Part | Time | What takes the time |
|---|---|---|
| compile, cold (app 31 s, command spotting 10 s, E2E harness 3 s) | 45 s | — |
| compile after touching `app/src/core/domain/mod.rs` | 6 s | 25 test binaries relinked |
| `engine_it` (command spotting, real keyword spotting model) | 43 s | `say` for every phrase, tests one at a time behind the engine lock |
| E2E harness unit tests | 21 s | `say` of 22 long paragraphs (2 languages × 11 recording conditions) |
| `chunk_cuts_it` | 18 s | `say` of 20 long paragraphs, then degrading and cutting them |
| `cucumber_integration` (51 scenarios, 41 `@serial`) | 15 s | real-time waits of the pipeline under test |
| `commandspotting_websocket_it` (stub services) | 11 s | the client's 10 s silence limit in the hang test |
| the other 24 binaries | ≈ 5 s | — |

The binaries run one after another. Run all at once (same build): **70 s** — `engine_it` 70 s, E2E harness 37 s,
`chunk_cuts_it` 36 s: the three that call `say` slow each other down.

Optimized test code changes little: `engine_it` 41 s, `chunk_cuts_it` 16 s, E2E harness 19 s, `cucumber_integration` 18 s
with `--release`. The time is not in Rust code.

### macOS `say`

| Call | Time |
|---|---|
| a short phrase ("press enter") | 1.06 s each, ten in a row 10.6 s |
| one paragraph of ≈ 60 words | 1.9 s |
| 22 such paragraphs at once | 15.7 s |

## `just check` — ≈ 22 min (1 310 s)

| Stage | Time |
|---|---|
| `_fmt` | 0.3 s |
| `_lint` (after an edit / cold) | 3 s / 26 s |
| `_test` | 113–123 s |
| `_build` (after an edit / cold) | 5 s / 43 s |
| `_e2e-smoke` | **1 092 s** |
| `_e2e-quick` | 55 s |
| `_refine-eval` | 31 s |

Earlier runs of the same recipe in `$TMPDIR/run-visible`: 472 s on 2026-10-12, 521–584 s on 2026-10-13,
1 097–1 175 s on 2026-10-14 — it doubled as the voice control tests landed.

### `_e2e-smoke` — 1 092 s

| Part | Time | What takes the time |
|---|---|---|
| release build of the test binaries | 36 s | 26 + 3 + 6 s in three `cargo test --release` calls |
| `commandspotting_websocket_it` (real service) | 25 s | one test keeps a silent session open 15 s |
| `voice_forms_it` | **397 s** | 83 recorded forms one after another, each followed by a fixed 4 s wait for more events |
| `voice_samples_it` | 76 s | a fixed 1.5 s wait after every recorded utterance |
| `placed_commands_it` | 16 s | `say` |
| `cucumber_e2e @smoke` (107 scenarios) | **538 s** | voice-control: 542 s on its own; file-transcription 9 s, speech-recognition 8 s, text-refinement 20 s |

`voice_control.feature @smoke` (75 scenarios) per scenario, run one at a time (`--concurrency 1`): **535 s** in total —
the same as with the harness's 4 concurrent scenarios (542 s). The steps block, and cucumber polls every scenario on
one thread, so the scenarios never overlap. Typical scenario 5–7 s: `say` ≈ 1 s, the fixed 4 s wait for more events
(twice in the scenarios that listen in both states), the service's processing. Longest: the hands-free session with
the user's recorded commands 52 s (real time by design), the app refusing to start without the command spotting
service 16.5 s, the hands-free session with long pauses 14 s.

### `_e2e-quick` — 55 s

WER on 10 recordings 6 s; the voice-control scenario "Ordinary speech triggers no command" on 20 recordings 47 s
(the fixed 4 s wait after each pass).

## `just full-check` — ≈ 27–28 min, red on `main`

| Stage | Time |
|---|---|
| `cargo clean`, then everything of `check` cold except `_e2e-quick` | ≈ 1 250 s (cold compile ≈ 150 s of it) |
| `_e2e-seams`: `chunked_transcription_it` | 138 s — fails (3 of 30 conditions, the known bug in `fix-dropped-phrase-at-word-pauses`) |
| `_e2e-seams`: `cucumber_e2e @seams` (26 scenarios) | 107 s — 9 of 26 fail: 7 with 10–20 % missing words, one phrase repeated at a seam, one phrase of 26 words missing (same change's acceptance bar) |
| `_e2e-bench`: WER on 100 recordings | 58–63 s |
| `_e2e-bench`: "Ordinary speech triggers no command" on 20 recordings | 27–100 s — failed once with `IO error: Resource temporarily unavailable (os error 35)` |
| `_refine-eval` | 31 s |

The `os error 35` is the client's 2 s write timeout: the test streams audio as fast as it can while the service is
still busy with what it already got, and the socket fills up.

## Services

A cold `just start-model-servers` on spare ports: 11.3 s (whisper-server 0.5 s, llama-server 5.8 s, rebuilding the
command spotting service for `-p` features 3.6 s, starting it 1 s). Every E2E recipe starts and stops the services it
did not find running: `check` would do it 3 times, `full-check` 4 times.

## Why the voice waits are idle time

The command spotting service handles one connection's messages in order on one thread, sends every event for a
binary frame before it reads the next message — the confirmation by the speech recognition service included — and
reads no clock (`command-spotting/src/server.rs`, `engine.rs`). A WebSocket ping sent after the last audio frame is
answered (tungstenite flushes the pong on its next read) only after every event for the audio before it was sent.
A fixed wait for "no more events" is therefore never needed.

# After the change

Measured on 2026-10-14 in the worktree of `chore/speed-up-checks`, same machine, models and running services as above.

| Check | Before | After |
|---|---|---|
| `just test`, warm | 113 s | **16 s** |
| `just test`, after an edit to `app/src` | 123 s | **23 s** |
| `just test`, cold (after `cargo clean`) | ≈ 160 s | 64 s |
| `just check`, fresh worktree (cold release build) | ≈ 1 310 s | **349 s** |
| `just full-check` | ≈ 1 650 s, red | ≈ 600 s, red on the same known seam failures |

`just check` by stage: fmt 0.3 s, lint 5 s, test 16 s, build 45 s (cold), e2e-smoke 199 s, e2e-quick 62 s, refine-eval 21 s.

`_e2e-smoke`: `voice_forms_it` 397 → 49 s, `voice_samples_it` 76 → 16 s, `commandspotting_websocket_it` 24 s, `placed_commands_it` 17 s, `cucumber_e2e @smoke` 538 → ≈ 80 s; no release build of the test binaries.

`just full-check` stops at the first failing stage, the known `chunked_transcription_it` failure: fmt 0.3 s, lint 28 s, test 64 s, build 44 s, e2e-smoke 152 s, then `chunked_transcription_it` 78 s. The stages after it, timed on their own: the `@seams` scenarios 53 s (107 s before; the same 9 of 26 fail with the same messages), `_e2e-bench` 146–166 s, `_refine-eval` 21–28 s — ≈ 590–615 s in total.

`just e2e-bench` three runs in a row: 166 s, 154 s, 146 s, all passing. With real overlap but before the `@accuracy` scenarios ran alone, 2 of 3 runs failed with "command spotting service stopped answering (10s without a reply)".

The speech cache holds 24 MB after a full run.

## Final run on the merged commit

After the hands-free sessions were made `@serial` and the listening passes went back to one after another: `just full-check` — fmt 0.3 s, lint 28 s, test 65 s, build 45 s, e2e-smoke 164 s, then the known `chunked_transcription_it` failure (3 of 30, 77 s); the stages after it on the same commit — e2e-quick 49 s, e2e-bench 142 s, refine-eval 32 s; ≈ 610 s with the `@seams` scenarios (53 s). `just e2e-bench` three runs in a row on that commit: 142 s, 141 s, 140 s, all passing.
