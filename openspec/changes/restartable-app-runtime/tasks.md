# Tasks

## 1. Core: pipeline activity and shutdown

- [ ] 1.1 Red: unit tests in `core/application/pipeline.rs` — the pipeline publishes Recording on start, Processing on stop through the paste, Idle after the paste and after a cancel, through a `PipelineActivitySink` port in `core/application/pipeline_activity.rs`; run them and see them fail
- [ ] 1.2 Green: the port (Idle, Recording, Processing; a no-op default) and `Pipeline::with_activity_sink`; the pipeline publishes on every transition, the focus wait included; 1.1 passes
- [ ] 1.3 Red: unit tests in `core/application/pipeline.rs` and `pipeline/voice_control.rs` — `PipelineInput::Shutdown` makes `run` return Ok while idle, cancels a recording in progress (cancel sound, nothing pasted) and stops voice control listening; run them and see them fail
- [ ] 1.4 Green: `PipelineInput::Shutdown` handled in `handle` and `handle_with_voice_control`; `run` returns Ok; 1.3 passes
- [ ] 1.5 `just test` and commit

## 2. Hotkey binding and re-targetable listener

- [ ] 2.1 Red: unit tests in `core/domain/hotkey.rs` — `HotkeyBinding` reads and replaces its `HotKey` from two threads; run them and see them fail
- [ ] 2.2 Green: `HotkeyBinding` (`Arc<Mutex<HotKey>>` behind `current()` and `rebind()`); 2.1 passes
- [ ] 2.3 Red: unit tests in `adapters/primary/ptt/rdev/mod.rs` for a pure `classify(event, binding, &mut ChordState)` — single key press and release, Escape, chord press and release by key and by modifier, a rebind to a second single key makes the first key signal nothing, a chord half held when the binding changes signals nothing on the chord's key; run them and see them fail
- [ ] 2.4 Green: `classify` with `ChordState` that resets when the binding differs from the one it saw last; `spawn_single` and `spawn_chord` become one thread calling it per event; `HotkeyListener::spawn` takes `HotkeyBinding`; the push-to-talk steps and `main.rs` pass a binding; 2.3 passes
- [ ] 2.5 `just test` and commit

## 3. Runtime module: router, forwarding overlay, state and observers

- [ ] 3.1 Red: unit tests in `runtime/router.rs` — a signal reaches the set sender, is dropped while none is set, and reaches the new sender after a swap; run them and see them fail
- [ ] 3.2 Green: `runtime/router.rs` (`SignalRouter` with the sender behind a mutex and its `hotkey-router` thread), `pub mod runtime` in `lib.rs`; 3.1 passes
- [ ] 3.3 Red: unit tests in `runtime/overlay.rs` — the forwarding indicator forwards every `VisualIndicator` call to the shared one; run them and see them fail
- [ ] 3.4 Green: `SharedVisualIndicator` over `Arc<Mutex<Box<dyn VisualIndicator>>>`; 3.3 passes
- [ ] 3.5 Red: unit tests in `runtime/observer.rs` — observers are told every state in order; a late observer is told the current state at registration; run them and see them fail
- [ ] 3.6 Green: `RuntimeState`, `RuntimeObserver`, `Observers` register and notify, the log observer; 3.5 passes
- [ ] 3.7 `just test` and commit

## 4. Runtime: assembler, wiring, start, reload, stop

- [ ] 4.1 Red: `app/tests/features/component/app_runtime.feature` with the scenarios of requirements 1–4 and 7 (every service answers; a service does not answer; the hotkey changes on reload; two reloads in a row; reload during a recording; reload while the pipeline waits for focus; reload with an invalid configuration; stop while idle; stop during a recording) and `app/tests/steps/app_runtime.rs` with an assembler of mocks, a fake hotkey feed into the binding and a recording observer; register in `steps/mod.rs`; run them and see them fail
- [ ] 4.2 Green: `runtime/assembler.rs` (`PipelineAssembler` trait, `PipelineParts`), `runtime/wiring.rs` (the former `connect_*` and `chunking_cfg` returning errors, the real assembler), `runtime/mod.rs` (`Runtime::start`, `reload`, `stop`, `state`, `observe`; the `runtime` thread owning the pipeline handle, the activity register, the router and the binding; reload waits for Idle, clears the router, sends `Shutdown`, joins, rebinds, assembles, starts; stop sends `Shutdown` at once; a rejected configuration is logged and keeps the pipeline); 4.1 passes
- [ ] 4.3 `just test` and commit

## 5. SIGHUP

- [ ] 5.1 Red: `app/tests/runtime_signal_it.rs` — a runtime with a mock assembler started from a temp config file reloads with the edited hotkey after the process sends itself SIGHUP; with a broken file it stays Ready and logs the reason; run them and see them fail
- [ ] 5.2 Green: `signal-hook = "=0.4.5"` in `app/Cargo.toml`, `runtime/signals.rs` with the `signals` thread (load, overlay, validate, reload); 5.1 passes
- [ ] 5.3 `just test` and commit

## 6. main.rs and architecture

- [ ] 6.1 Red: `app/tests/architecture.rs` — `src/core` never imports `crate::runtime`, and no file outside `src/runtime/` and `src/main.rs` imports `crate::adapters::secondary` into a pipeline assembly; run it and see it fail (or pass for the right reason and tighten it)
- [ ] 6.2 Green: `main.rs` keeps `Cli::parse`, config path, the setup wizard, load, overlay, validate, logging and file mode; hotkey mode creates the overlay, the runtime (with `Preflight` inside `start`), the log observer and the signals thread, exits 2 on a start error, then enters `NSApplication.run()`; `cargo run` starts as before
- [ ] 6.3 `just test` and commit

## 7. Docs and glossary

- [ ] 7.1 `docs/ubiquitous-language.md`: Runtime, Runtime state, Hotkey binding, Signal router, Pipeline activity, Shutdown; `docs/project-map.md`: the `runtime/` module; `docs/architecture/overview.md`: the runtime in the component view; `README.md`: "Reload the configuration without a restart" (`kill -HUP`)
- [ ] 7.2 `just test` and commit

## 8. Release

- [ ] 8.1 Bump `version` in `app/Cargo.toml` to 0.25.0 (or the next minor above `main`)
- [ ] 8.2 `just check` once and commit
- [ ] 8.3 Manual run by the user: with `just start-model-servers` running, `cargo run --release -p voicepuncher`; dictate; edit `hotkey.key` in `config.toml`; `kill -HUP PID`; the log shows `runtime: reloading configuration` then `runtime: ready (hotkey …)`; dictate with the new key, the old key does nothing; paste the log
