## Context

The pipeline owns one `Box<dyn Transcriber>` and hands it to the live transcription worker of every dictation; the worker returns it with its result. A worker that dies (panic) drops it, and the next dictation panics on `transcriber already taken` / `transcriber not available`; the pipeline thread's error ends the process with `process::exit(1)`. `stop_recording` and `discard_recording` wait for the worker's result with an unbounded `recv()`, on the pipeline thread that reads every input.

## Goals / Non-Goals

**Goals:** a dead worker loses only its dictation; the wait for a stopped dictation's result is bounded; a failed pipeline does not end the process.
**Non-Goals:** the duplicated refine/paste paths (`slim-pipeline-module`), the command spotting service's stalls (`command-spotting-stall-handling`), a change to the `Transcriber` port.

## Decisions

1. **Shared transcriber.** The pipeline keeps `Arc<Mutex<Box<dyn Transcriber>>>` (`SharedTranscriber`) for its whole life; each live worker gets a clone and locks it per chunk. The `TranscriberSource` hand-off, `dropped_worker`, `reclaim_transcriber` and the `transcriber` field of `LiveSessionResult` go. A lock poisoned by a worker's panic is taken over (`into_inner`), so the next dictation transcribes. Alternative rejected: `Arc<dyn Transcriber>` with `&self` — 13 implementors would need interior mutability for the same effect; rebuilding from the assembler — the pipeline has no assembler.
   - A worker still on a hung request holds the lock; the next dictation's worker waits for it, and the adapter's own per-request deadline (at most `timeout_secs`) ends that request. This is the same wait the dropped-dictation hand-off had.
2. **A dead worker ends the dictation with a notice.** A disconnected result channel notes `DictationLost(SpeechRecognition)` before the cycle ends, as a request that failed on every attempt does.
3. **Bounded stop wait.** `Pipeline::with_stop_wait(Duration)` sets the bound; `stop_recording` and `discard_recording` use `recv_timeout`. Wiring derives it from the configuration: `openai::longest_wait(timeout_secs)` (every attempt at its longest deadline plus the pauses between them) + 2 s; the default without wiring is 184 s, that derivation from the default 60 s timeout rounded up. No new configuration key. Past the bound: log, note `DictationLost(SpeechRecognition)`, stop the recorder, end the cycle; the worker's late result goes nowhere. Inputs that arrive during the wait stay in the input channel and are handled next, so a hotkey press is not lost.
   - Risk: a dictation whose worker is several chunks behind could hit the bound; chunks of 30 s transcribe in about a second, so a backlog that long means the service is failing anyway.
4. **Fatal pipeline end.** The pipeline thread runs `pipeline.run` inside `catch_unwind`; on an error or a panic it calls `Pipeline::stop_after_failure` (recorder stopped, listening off, overlay hidden, activity Idle — each step itself guarded, as the panic may have left the parts half-used), logs `fatal: pipeline stopped: <reason>`, and sends `Command::PipelineFailed { generation, reason }` to the supervisor, which owns the command channel's sender from its start. The supervisor ignores a message from a pipeline it already replaced (`generation`), joins the thread, and starts a new pipeline from the same configuration (state Starting → Ready); when the failed pipeline had started less than `RESTART_WINDOW` (30 s) after a previous fatal restart, it reports Failed and sets `failed`, as a pipeline that cannot start does: no other pipeline starts in this process. Services keep running; quitting stops them as before.

## Risks / Trade-offs

- A panic inside the transcriber while it holds the lock leaves the adapter as the panic left it; the HTTP adapter keeps no state across requests besides the copy pool, which releases a copy on drop.

## Test levels

| Scenario | Level |
|---|---|
| The worker dies mid-dictation | component (`speech_recognition.feature`) |
| The speech recognition request never answers | component (`speech_recognition.feature`) |
| The pipeline panics once | component (`app_runtime.feature`) |
| The restarted pipeline panics again at once | component (`app_runtime.feature`) |
