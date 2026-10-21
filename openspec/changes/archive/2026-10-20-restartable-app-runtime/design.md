# Design

## Context

`main.rs` owns the pipeline, the hotkey listener, the overlay and the spotter; nothing can be undone. `rdev::listen` never returns. `Pipeline::run` ends only when every input sender is dropped, but the pipeline owns the spotter, whose worker holds a sender. The architecture test forbids `Atomic*` under `src/core`.

## Decisions

**D1 — `app/src/runtime/` beside `adapters`, outside `core`.** Composition code imports adapters and ports, which `core` forbids and `adapters` (feature/tech only) does not fit. `main.rs` calls only the runtime and file mode.

**D2 — The hotkey listener starts once and is re-targeted; the pipeline is replaced.** The rdev thread lives as long as the process and reads the current hotkey binding (a lock around `HotKey`) on every event; the chord state resets when the binding changed since the last event. Every pipeline part is rebuilt from the new configuration.

**D3 — A router the runtime swaps; `Shutdown` ends a pipeline.** The router holds the current input sender behind a mutex; the runtime sets it before a pipeline starts and clears it before stopping one (signals meanwhile are dropped with a debug line). `PipelineInput::Shutdown` makes the pipeline cancel a recording in progress, stop voice control listening (closes the microphone, joins the worker) and return from `run`. Rejected: closing the channel — deadlocks with voice control on.

**D4 — Restart waits for Idle; stop does not.** The pipeline publishes Idle / Recording / Processing (recognition, refinement, focus wait, paste) through a port in `core/application`; the runtime implements it with an atomic. `reload` waits for Idle without limit, state Restarting visible; `stop` sends `Shutdown` at once (quit is the user's decision, as `menu-bar-presence` needs).

**D5 — One overlay and one level register per process.** Made once in `main` with the main thread marker; pipelines get a forwarding `VisualIndicator` over `Arc<Mutex<Box<dyn VisualIndicator>>>`; every new recorder gets the same level register.

**D6 — State through an observer trait on the runtime thread.** `RuntimeState` (Starting, Ready { voice_control }, Restarting, Stopped, Failed(reason)); `RuntimeObserver::state_changed` is called synchronously, with the current state at registration; GUI observers dispatch to the main thread themselves.

**D7 — SIGHUP through `signal-hook` `=0.4.5`.** A `signals` thread iterates the signal, loads, overlays and validates the file, then calls reload.

**D8 — Failure keeps exit code 2.** `Runtime::start` returns the error (service silent, permission missing through `Preflight`); `main` exits 2 as today; the state Failed carries the reason.

**D9 — A `PipelineAssembler` trait builds the parts.** It turns a configuration into transcriber, refiner, recorder, paster, sounds, spotter and archive, probing the services; `wiring.rs` is the real one, component tests supply mocks and a fake hotkey feed. Rejected: real adapters only — testable only with live services.

## Risks / Trade-offs

- [A reload probes the transcriber again, up to its connect timeout] → Restarting is visible.
- [Two SIGHUPs at once] → reloads are serialised on the runtime thread; the second restarts again with the newer file.
- [`menu-bar-presence` edits `main.rs` in parallel] → the change merged second resolves the conflict from both designs; no dependency.

## Test levels

| Scenario | Level |
|---|---|
| Every service answers; A service does not answer; The hotkey changes on reload; Two reloads in a row; Reload during a recording; Reload while the pipeline waits for focus; Reload with an invalid configuration | component `app_runtime.feature` (assembler with mocks, fake hotkey feed) |
| An observer sees every state in order; a late observer learns the current state | unit tests in `runtime/observer.rs` (`Observers`) and `runtime/mod.rs` (a runtime started, reloaded and stopped) |
| Hang-up signal with an edited configuration file; Hang-up signal with a broken configuration file | adapter integration `runtime_signal_it.rs` (SIGHUP to itself, temp config) |
| Stop while idle; Stop during a recording | component `app_runtime.feature` |
| Single key rebinds; chord half pressed when the binding changes | unit tests in `adapters/primary/ptt/rdev/` |
| Router swap and drop; shutdown | unit tests in `runtime/router.rs`, `core/application/pipeline.rs` |
| Activity transitions | unit tests in `core/application/pipeline.rs` |
| `core` never imports `runtime` | `app/tests/architecture.rs` |
| A real dictation after a `SIGHUP` hotkey change | manual run by the user |
