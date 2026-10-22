## Context

See proposal.md § Why. `Pipeline::new` is called from 22 files (wiring, main, every component step module, the E2E harness); the three ports with default bodies have 20 implementors besides the mocks. The runtime reads the pipeline's activity through `ActivityRegister` (an atomic) polled every 20 ms. The user's manual-run check reads the log lines `pipeline recv …`, `pasting…` and `stop→paste: … total=…ms`: they stay word for word.

## Goals / Non-Goals

**Goals:** the pipeline files within the budgets, no duplicated path, the invariants in types, behavior and log lines unchanged.
**Non-Goals:** asynchronous processing; the budgets of `runtime/mod.rs` and `main.rs`.

## Decisions

1. **The dictation is a value.** `dictation: Option<Dictation>` replaces `state`, `live_session`, `command_words`, `pending_notice` and `trace`: one value carries a dictation's live session, command words, pending notice and archive trace from start to close. `is_recording()` is `is_some()`; `stop_recording` takes it out and processing works on it, so `Transcribing`/`Refining`/`Pasting` and the two tests assigning them go. Rejected: an enum `Idle | Recording(..)` — the same with more matching.
2. **One processing path.** Both entries (a recording transcribed on stop, a live session merged) yield a `RawTranscript` or an end without text, plus a `StageClock` of named stage durations in order; `finish(dictation, raw, clock)` refines, delivers, ends the overlay, logs `stop→paste: <stages> total=…ms` and closes the trace; `end_without_text(dictation, outcome, clock)` is the single no-text end. `abandon(dictation, Awaits::{WorkerResult, Nothing})` replaces `drop_recording`/`discard_recording`; `close_dictation(spoken)` replaces the StopDictation/PressEnter twins.
3. **Twelve fields.** The six ports, `cfg` (gaining `stop_wait` and `shows_refinement`), `dictation`, `focus: Option<FocusWait>`, `inbox` (receiver and pending queue), `voice`, `archive`, `activity`. `Pipeline::new(parts: PipelineParts, cfg)` moves the six ports into their fields; the builder methods stay.
4. **`Voice` is `Off` or `On(VoiceControl)`.** The dispatch matches once; `VoiceControl` methods never `expect`; `follow_dictation`, `heard_since`, `turn_listening_off` on `Voice` do nothing when `Off`.
5. **Ports without defaults.** The eight methods lose their bodies; each implementor states its behavior: the file recorder and the file-mode overlay (`visualfeedback/quiet.rs`) write their no-ops out; a test double invokes one of three macros in `app/src/test_doubles.rs` (`records_whole_buffers_only!`, `shows_no_voice_control_signs!`, `holds_no_heard_sound!`) inside its `impl`, so the duplicated-lines ratchet does not grow; the component scenarios that drive the pipeline by its input channel share `QuietSpotter` and `NoKeys` from `app/tests/support/quiet_voice_control.rs`, and each listed step file keeps its doubles in a `doubles.rs` submodule so it shrinks. Rejected: smaller ports — three traits for eight one-line bodies.
6. **Errors and output.** `stop_recorder_quietly(why)` logs a failed stop at warn; a worker result nobody receives logs at debug. `run_once` returns the pasted `CleanedText`, `run_file_mode` prints it; the hotkey-mode `eprintln!` goes (no terminal).
7. **`main` returns an exit code.** `fn main() -> ExitCode` calls `run(cli) -> Result<ExitCode>`; an error is printed once at the top and maps to 2; the download and file modes keep their codes.
8. **The runtime waits on a condvar.** `ActivityRegister` holds the activity under a mutex with a condvar; `wait_idle(for)` returns when `Idle` is published or after `for`. `wait_for_idle` blocks on it between command checks: an idle pipeline wakes the restart at once, a stop is seen within the poll interval, the grace wait after the detach uses the same call. `end_pipeline(config)` (shut down, rebind, keep the configuration) serves `reload` and `wait_for_models`.
9. **Files.** `pipeline.rs` keeps the struct, `new`, the builders and the run loop; `pipeline/dictation.rs`, `pipeline/recording.rs` (start, stop, cancel, abandon), `pipeline/processing.rs` (transcribe, refine, deliver, finish, `StageClock`), `pipeline/live_worker.rs`; the unit tests move to `pipeline/tests/` on one `PipelineUnderTest` builder with no-op defaults, asserting the pasted text, the overlay calls and the published activity.
10. **Baselines.** The last group drops the entries of units gone or shrunk (`just size-baseline`, `just analyzers-baseline`); none is added or raised.

## Dependencies

`fix-load-sensitive-confirm-tests`: the confirm tests of the command spotting service failed in this change's `just test` on 2026-10-22 under another session's `full-check`; a flake is a failure, so the branch lands only after that fix (declared in `orchestration.yaml` once that change has started; `kit` refuses a dependency on an unstarted change). Otherwise none: `fast-test-build` moves `app/tests/*_it.rs` into `app/tests/it/` and rewrites the baselines, files this change also touches (the constructor sweep, the baseline entries), but reworks nothing this change builds on; the one merged second resolves the path moves.

## Risks / Trade-offs

- [22 constructor sites, 20 implementors] → compiler-driven sweeps, one commit each, no logic in them.
- [A log line changes wording] → the pipeline unit tests assert the `stop→paste` line through a capturing logger.
- [The grace wait after the detach still relies on time] → same 20 ms, a condvar wait instead of a sleep; the race it covers is out of scope.

## Test levels

| Scenario | Level |
|---|---|
| Every dictation path behaves as before | component: the existing `.feature` files (processing-stages, failure-notices, voice-control, live-transcript, dictation-archive, text-insertion, app-runtime), unchanged |
| Cues, overlay, activity, timings, abandon with and without a worker | unit (`pipeline/tests/`, rewritten on the builder) |
| The restart wakes as soon as the pipeline is idle | unit (`runtime/activity.rs`) |
| A configuration error exits with code 2 and one message | adapter integration (`cli_it.rs`, existing) |
