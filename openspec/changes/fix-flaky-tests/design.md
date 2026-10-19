## Context

Failures from 14–18 Oct were traced in run logs and transcripts. Most were TDD reds, defects since fixed, or checks reusing the user's stale service on port 8082 (fixed by isolate-check-services). Seven causes remain; the proposal lists them.

## Decisions

### 1. Corpus default in a worktree

`justfile` defaults `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus` when the worktree has no `manifest.jsonl` (the first entry of `git worktree list`). A test that needs a corpus whose manifest is missing reports itself gated and passes, as tests gated on a missing env var already do. Rejected: adding `_require-corpus` to `_e2e-smoke` — it would fail every worktree check instead of fixing it.

### 2. Stub services accept before they time out

The stub WebSocket servers in `commandspotting_client_it.rs` and `commandspotting_reconnect_it.rs` set the 10 ms read timeout after `tungstenite::accept`, so a handshake request arriving later than 10 ms is not cut.

### 3. Tests of the real command spotting service wait for what they expect

`commandspotting_websocket_it.rs` stops collecting after 2 s of quiet, shorter than the service may legitimately stay silent (a confirmation may take up to 3 s, settling two of them). It instead waits for the first command up to 10 s, then 2 s more to see no extra one. Binaries that drive the real command spotting service (`commandspotting_websocket_it`, `voice_forms_it`, `voice_samples_it`) join the nextest group `command-spotting-models` (one at a time), so they do not load one confirmation service from many processes at once.

### 4. A busy client is not a silent service

Fixed by fix-spotting-lost-at-check-start, which started earlier and owns the client's silence rule; not repeated here.

### 5. Checks keep their audio when they wait

The engine asks a check of a running utterance (early head, head, window, spotted check) only while no other check waits. Head and spotted checks are deferred but then cover the audio up to the moment asked, not the moment due; windows are skipped outright. Which audio a check hears therefore depends on how fast the confirmation service answers, which under GPU load loses a command or carries one out twice. Now a check due while another waits is queued with the audio range it was due for (`from`, `to` in stream samples), and asked with exactly that range once the other is answered; a later window replaces a queued one of the same utterance, so the queue stays short. The answers still arrive later under load, but every check hears the same audio as it would have on time.

E2E steps feed audio on an absolute schedule (`deadline += FRAME`) instead of sleeping one frame after each send, so slow sends do not stretch the speech.

### 6. Seam repeats count only spoken pairs

The seam check in `e2e-test/src/speech.rs` counts a word pair as repeated only when it occurs in the speech and the transcript holds it more often than the speech does; a pair the recognizer misheard twice is a recognition error, measured by WER, not a seam repeat.

### 7. No sampling fallback for dictation

Dictation requests send `temperature_inc=0`, as command confirmations already do, so whisper-server never retries an unsure window at a higher temperature. The change ships only if WER on the 100-recording sample (`full-check` runs it, since the request is on the recognition path) is not worse than on `main`; otherwise this decision is dropped and the proposal amended.

## Risks

- Queued checks answer later than skipped ones under load; a command may be carried out later, never lost or doubled.
- Without the sampling fallback, a hard window may come back with a repetition loop that the fallback used to break; the hallucination filter and the 100-recording WER guard catch it.

## Test levels

| Scenario | Level |
|---|---|
| A command is carried out once while confirmations come back slowly | adapter integration, `command-spotting/tests/placed_commands_it.rs` (engine with a delayed hearing) |
| A chain is carried out the same at any confirmation speed | adapter integration, `command-spotting/tests/placed_commands_it.rs` |
| Hard audio transcribed twice gives the same text | unit (request carries no fallback) and adapter integration, `transcription_openai_it.rs` |
