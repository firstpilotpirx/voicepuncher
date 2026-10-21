## Why

`app/src/core/application/pipeline.rs` (1 882 lines) and `pipeline/voice_control.rs` (1 310) are the hub of the app and the clearest case of accretion in the audit of 2026-10-22: `Pipeline` holds 19 fields with its six ports `pub`; `run_transcribe_refine_paste` (`:562–613`) and `run_refine_paste` (`:689–731`) repeat refine → deliver → timing → `close_trace`; `drop_recording` (`:440–456`) and `discard_recording` (`:459–477`) are near-twins; StopDictation and PressEnter repeat `closing`/`truncate_at`/`stop_recording` (`voice_control.rs:185–196`); `PipelineState::{Transcribing, Refining, Pasting}` are set and cleared inside one synchronous handler, so no input can observe them, yet `step()` matches on them and tests at `:1480` and `:1505` assign them by hand; `VoiceControl` is an `Option` reached through `expect("voice control is on")` (`voice_control.rs:76–78`); seven port methods have default no-op bodies (`audio_recorder.rs:24–32`, `visual_indicator.rs:22–31`, `command_spotter.rs:21–23`); `let _ = self.recorder.stop()` swallows errors at four sites; `eprintln!` lives in the core (`:517, :551`); the same mock set-up opens ~10 tests; `main()` is 165 lines with 11 copies of `unwrap_or_else(… exit(2))`; `runtime/mod.rs` polls every 20 ms in `wait_for_idle` and repeats the shut-down/rebind block in `reload` and `wait_for_models`.

## What Changes

- One processing path: refine → deliver → timing → trace in one function with the timing fields as a value; one "end the recording without text" path; one "close a dictation on a command" path.
- Only states an input can observe stay (`Idle`, `Recording`, and a processing state only if processing becomes asynchronous — decided in design with `fix-transcriber-loss-and-stop-hang`).
- `VoiceControl` is a type that holds its own state; the pipeline calls it without an `Option` and an `expect`.
- Ports have no default bodies: each adapter implements what the port names, or the method moves to a smaller port.
- Errors are logged or returned, never `let _`; terminal output leaves the core (the CLI adapter prints).
- `Pipeline`'s fields are private; its construction is a builder or a `PipelineParts` value so the constructor has ≤ 5 parameters.
- `main()` becomes `fn run() -> Result<ExitCode>`; the runtime waits for idleness on a condvar or channel, and `reload`/`wait_for_models` share one shutdown step.
- The pipeline tests share one builder for the mock set-up and assert observable effects (what was pasted, what the overlay showed), not the private state.
- Each file ends under the budgets of `enforce-size-and-complexity-budgets` or is split along the seams the design names (recording session, processing, voice control, failure notices).

Current stage: PoC. Target stage: PoC.
Scope: `pipeline.rs`, `pipeline/*.rs`, the port traits with default bodies, `runtime/mod.rs`, `main.rs`, their unit and component tests.
Acceptance bar: every component scenario and E2E smoke passes unchanged; no duplicated path remains (the duplication detector reports none in these files); `Pipeline` under 12 fields; no function over the budget; no `expect` on an invariant a type can carry.
Out of scope: behavior changes (none — the two bugs are `fix-transcriber-loss-and-stop-hang`, which lands first).

## Capabilities

### Modified Capabilities

- none (a refactor; the specs stay as they are).

## Impact

- `app/src/core/application/pipeline.rs`, `pipeline/`, `audio_recorder.rs`, `visual_indicator.rs`, `command_spotter.rs`, `app/src/runtime/mod.rs`, `app/src/main.rs`, `app/src/adapters/**` that implement the widened ports, `app/tests/steps/*`.

## Open points

- Whether processing becomes asynchronous (a worker per dictation, the pipeline thread free) in this change or the states are collapsed and asynchrony waits; proposed: collapse now, asynchrony only if the stop-hang fix needs it.

## Release

SemVer: no release — refactor without behavior change.
Target version: unchanged.
