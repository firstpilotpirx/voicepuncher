## Why

Two failure paths in the dictation pipeline turn a service hiccup into a frozen or crashed app (audit of 2026-10-22, verified in the code):

- The single `Box<dyn Transcriber>` is handed to the live transcription worker and back through channels. When the worker dies (any panic in that thread) `reclaim_transcriber` (`app/src/core/application/pipeline.rs:352–356`) and the `Err` branch of `stop_recording` (`:407–412`, "worker channel closed unexpectedly") leave `self.transcriber == None`; the next recording hits `panic!("transcriber already taken")` (`:347`) or `.expect("transcriber not available")` (`:526, :629`); the pipeline thread's panic is answered by `std::process::exit(1)` in `app/src/runtime/mod.rs:255–258`. One dead worker crashes the app on the *next* dictation, with a misleading message.
- `stop_recording` blocks the pipeline thread — the only consumer of hotkey and voice inputs — on `session.results_rx.recv()` with no timeout (`pipeline.rs:368`). A speech recognition request that hangs freezes the app: no hotkey, no cancel, no overlay change, for as long as the HTTP timeout and forever when the worker hangs before sending.

## What Changes

- A dead worker never takes the transcriber with it: the pipeline keeps a transcriber it can use again (shared, or rebuilt from the assembler on loss), the `TranscriberSource`/`dropped_worker` hand-off goes away, and a worker that died ends the dictation with a failure notice ("speech recognition failed") and leaves the app ready for the next one.
- Stopping a recording waits for the worker's result for a bounded time (the longest one request may take with every retry at the configured `[transcription] timeout_secs`, plus 2 s for the drain); past it the dictation ends with a failure notice, the recorder stops, the overlay hides, and the pipeline reads inputs again; a hotkey press during the wait is not lost.
- A panic in the pipeline thread stops the pipeline in order (services stopped, overlay hidden, log line) instead of `process::exit(1)` from a non-main thread, and the runtime starts a new pipeline; one that fails again within 30 s of that restart leaves the runtime Failed, the process still running.
- Component scenarios reproduce both paths first (TDD red): a worker that dies mid-dictation, a request that never answers.

Current stage: PoC. Target stage: PoC.
Scope: `Pipeline` ownership of the transcriber, the stop wait, the fatal path of the runtime; the component feature `app_runtime` / `speech_recognition`.
Acceptance bar: with the worker killed mid-dictation the next dictation pastes; with the recognition service paused on stop the app shows a failure notice within the stop-wait bound (every attempt at the configured timeout, plus 2 s) and the hotkey works right after; no `panic!`/`expect` on the transcriber path.
Out of scope: the duplicated refine/paste paths and the fictional pipeline states (`slim-pipeline-module`); the command spotting service's own stalls (`command-spotting-stall-handling`).

## Capabilities

### Modified Capabilities

- `speech-recognition`: a dead worker or an unanswered request ends the dictation with a notice, the app stays usable.
- `app-runtime`: a fatal pipeline error stops in order.

## Impact

- `app/src/core/application/pipeline.rs`, `pipeline/*.rs`, `app/src/runtime/mod.rs`, `app/src/runtime/wiring.rs`; `app/tests/features/component/{app_runtime,speech_recognition}.feature` and steps; `openspec/specs/{speech-recognition,app-runtime}`.

## Open points

- Shared `Arc<dyn Transcriber>` (the adapter is already thread-safe over HTTP) versus rebuilding from the assembler; proposed: `Arc`, the smaller change.
- The bound on the stop wait: derived from `[transcription] timeout_secs` (every attempt with its retries, plus 2 s), or its own key; decided: derived, no new key.

## Release

SemVer: PATCH — bug fixes.
Target version: 0.36.1 (next PATCH above `main` at merge; `service-reliability-review` takes 0.36.0).
