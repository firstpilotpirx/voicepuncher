## Why

`app/src/core/application/pipeline.rs` (1 614 code lines, budget 300) is the hub of the app and the clearest accretion in the audit of 2026-10-22: `Pipeline` has 21 fields, six of them `pub` ports nothing outside reads; two functions repeat refine → deliver → timing → trace, two end a recording the same way, two voice commands close a dictation the same way; the states `Transcribing`, `Refining`, `Pasting` live inside one synchronous handler, so no input observes them; `VoiceControl` is an `Option` behind `expect`; eight port methods have default no-op bodies; `let _ = self.recorder.stop()` swallows errors at six sites; `eprintln!` lives in the core; `main()` repeats `exit(2)` eight times; the runtime polls every 20 ms for idleness and repeats its shut-down block twice.

## What Changes

- One processing path with the timings as a value; one "end the recording without text" path; one "close a dictation on a command" path.
- Only `Idle` and `Recording` stay; processing remains synchronous.
- `VoiceControl` holds its own state, off or on, without `Option` or `expect`.
- Ports have no default bodies: every adapter and test double implements what the port names.
- Errors are logged or returned, never `let _`; the CLI prints the pasted text in file mode, nothing prints in hotkey mode.
- `Pipeline`'s fields are private and at most 12; it is built from a `PipelineParts` value and its config.
- `main()` becomes `fn run() -> Result<ExitCode>`; the runtime waits for idleness on a condvar; `reload` and `wait_for_models` share one shut-down step.
- The pipeline tests share one mock builder and assert observable effects, not private state.
- `pipeline.rs` splits along its seams (recording, processing, live worker, tests) into files within the budgets; `runtime/mod.rs` and `main.rs` only shrink.

Current stage: PoC. Target stage: PoC.
Scope: `pipeline.rs`, `pipeline/*.rs`, the three ports with default bodies and their implementors, `runtime/{mod,activity}.rs`, `main.rs`, their tests.
Acceptance bar: every component scenario and E2E smoke passes unchanged; the duplication count of `metrics` shrinks; `Pipeline` at most 12 fields; no new or grown entry in `size_baseline.toml`.
Out of scope: behavior changes; asynchronous processing; splitting `runtime/mod.rs` and `main.rs` to the budget.

## Capabilities

### Modified Capabilities

- none (a refactor; the specs stay as they are).

## Impact

- The files in scope, the adapters and test steps implementing the three ports.

## Release

SemVer: no release — refactor without behavior change.
Target version: unchanged.
