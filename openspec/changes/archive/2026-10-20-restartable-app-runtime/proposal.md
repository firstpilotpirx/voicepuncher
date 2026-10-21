## Why

voicepuncher is becoming a menu bar application (see the sibling changes `menu-bar-presence`, `permissions-gate-window`, `settings-window`, `local-services-runtime`). Every one of them needs the same thing the code base does not have today: a way to start the dictation pipeline when its prerequisites are met, to stop it, and to start it again with new settings while the process, its menu bar icon and its windows stay alive.

Today `app/src/main.rs` is the whole life of the process: it parses the command line, loads `~/.voicepuncher/config.toml`, runs the setup wizard, runs the permission preflight (which exits with code 2), builds every adapter, spawns the pipeline thread and enters `NSApplication.run()`. Nothing in it can be undone:

- the pipeline thread ends only when the process does; a changed hotkey, sound, language or service address needs a process restart;
- the hotkey listener (`rdev::listen` in `adapters/primary/ptt/rdev/`) is bound to one key at spawn time and can never be stopped (rdev has no stop call), so the hotkey cannot change while the process runs;
- the overlay (`MacosVisualIndicator`) is created once on the main thread and owned by the pipeline, so a second pipeline cannot show it;
- the command spotting client (`WebSocketCommandSpotter`) owns a sender into the pipeline's input channel and is stopped only by dropping the process;
- `connect_transcriber`, `connect_refiner` and `connect_command_spotter` call `std::process::exit(2)` when a service does not answer, so nothing above them can decide to wait instead.

The settings window will need "Save" to apply without a relaunch (the user asked for a window with every option, applied in place). The permissions gate will need to start the pipeline only once the three permissions are granted and to stop it when one is revoked. The local services runtime will need to start the pipeline when its services are ready. All three are the same operation: stop the pipeline if it runs, build it again from the current configuration, start it, and tell the user interface what state it is in. This change builds that operation once, so the three changes above only call it.

## What Changes

- A `Runtime` module owns the dictation pipeline and every adapter it needs. It starts the pipeline from a configuration, stops it, and restarts it from a new configuration while the process keeps running. `main.rs` shrinks to: parse the command line, load and validate the configuration, set up logging, run file mode when `--input-file` is given, otherwise create the runtime, start it and enter `NSApplication.run()`.
- The runtime has an observable state: Starting, Ready (with whether voice control is on), Restarting, Stopped, Failed with a reason. User interface parts (the menu bar icon, the settings window, the permissions gate of later changes) register as observers and are told about every state change. This change ships the state and the observer registration; it has no graphical observer yet, the log is the only observer.
- Sending `SIGHUP` to the process (`kill -HUP PID`) reloads `config.toml` and restarts the pipeline with it, without a relaunch. The log shows `runtime: reloading configuration from PATH`, then `runtime: ready (hotkey Right Option, voice control off)`. A configuration that does not load or validate is logged (`runtime: configuration rejected: REASON; the running pipeline keeps its settings`) and the running pipeline keeps running with its old settings. This is the developer's way to try a config edit without restarting; the settings window will call the same reload.
- A restart never cuts a dictation: when the pipeline is recording or processing, the runtime waits for it to finish (state Restarting, log `runtime: restart waits for the dictation in progress`), then restarts. The hotkey listener stays alive across restarts; it is started once per process and re-targeted to the new hotkey.
- The hotkey can change without a relaunch: the hotkey listener matches key events against the current hotkey binding, which the runtime updates on reload. A press of the old key after a reload does nothing; a press of the new key records.
- Services that do not answer at start no longer end the process from inside the wiring: the runtime reports the state Failed with the reason (`speech recognition service at URL does not answer`) and the process exits with code 2 as today, so the command line behaviour of this version is unchanged. Later changes (`local-services-runtime`) turn Failed into waiting for the services.
- Everything the user sees in a dictation stays as it is: the same hotkey behaviour, overlay, sounds, paste, voice control, archive, failure notices. File mode (`--input-file`) does not use the runtime and is untouched.

Current stage: PoC. Target stage: Prototype (the composition root becomes a restartable runtime the GUI changes build on).
Scope: the runtime module and its state; re-targetable hotkey listener; the pipeline's observable activity (idle, recording, processing); shared overlay across restarts; SIGHUP reload; `main.rs` reduced to the command line, file mode and the runtime; documentation and glossary.
Acceptance bar: with the app running, editing `config.toml` (another hotkey, another sound) and sending `SIGHUP` applies the edit within 2 s and the next dictation uses it; a reload sent during a dictation applies after the paste; a broken config edit is logged and the app keeps dictating with the old settings; `just test` green with the new component scenarios; the user pastes a healthy log of a manual run with a hotkey change applied by `SIGHUP`.
Out of scope: any window, menu bar icon or dialog (later changes); starting services (`local-services-runtime`); the permissions gate (`permissions-gate-window`); writing `config.toml` (`settings-window`); the setup wizard and preflight stay as they are until `permissions-gate-window` and `retire-cli-setup-wizard` replace them.

## Capabilities

### New Capabilities

- `app-runtime`: the life of the application: the pipeline starts from the configuration, stops, restarts with new settings without a relaunch; the runtime state is observable; the hotkey rebinds without a relaunch; a restart waits for the dictation in progress; a rejected configuration keeps the running pipeline; `SIGHUP` reloads the configuration.

### Modified Capabilities

- none. `push-to-talk`, `voice-control`, `visual-feedback` keep every requirement; their behaviour must be unchanged after the move into the runtime.

## Requirements to specify

The specs stage writes `specs/app-runtime/spec.md` with these requirements (names are suggestions; keep the meaning):

1. The pipeline starts from the configuration: when the runtime starts with a valid configuration and every service answers, the pipeline runs with that configuration and the state becomes Ready; a service that does not answer makes the state Failed with the service named.
2. A new configuration applies without a relaunch: when the runtime is asked to reload with a new valid configuration, the pipeline runs with the new configuration (the new hotkey records, the old one does not; the new sounds play) and the process is the same.
3. A restart waits for the dictation in progress: when a reload arrives while the pipeline records or processes, the dictation finishes and is pasted, then the pipeline restarts; the state is Restarting meanwhile.
4. A rejected configuration keeps the running pipeline: when a reload arrives with a configuration that does not load or validate, the pipeline keeps running with its settings and the reason is logged.
5. The runtime state is observable: every observer registered is told of every state change (Starting, Ready, Restarting, Stopped, Failed), in order, including the state at registration.
6. SIGHUP reloads the configuration: when the process receives SIGHUP, the runtime reloads the configuration file it was started with and applies it as in requirement 2.
7. Stopping the runtime ends the pipeline: when the runtime stops (quit), the pipeline ends, the command spotting client stops, and no recording is left open; the hotkey listener is the only thread that may remain until the process ends.

## Implementation guide

The order below is the order of the task groups; each behaviour is test-first (AGENTS.md § TDD).

1. **Pipeline activity.** Add a small port in `core/application` through which the pipeline publishes what it is doing (Idle, Recording, Processing), set on every state transition it already logs (`pipeline recv ... in state ...`); the runtime module implements it with an atomic (the architecture test forbids atomics under `core`) and reads it to decide whether a restart must wait. Unit-test the transitions.
2. **Re-targetable hotkey listener.** `HotkeyListener::spawn` takes a shared hotkey binding (a domain type that holds the current `HotKey` behind a lock and can be read per event and replaced by the runtime) instead of a fixed `HotKey`. In the rdev adapter, extract the pure classification of an rdev event against a binding into a function (single key or chord) and unit-test it with synthetic events; the thread calls it on every event with the binding's current value. The thread is spawned once per process and never joined (rdev's `listen` cannot return).
3. **Signal router and shutdown.** Hotkey signals go to a router that holds the current pipeline input sender behind a lock; the runtime swaps the sender on every restart and clears it while no pipeline runs (signals are then dropped with a debug log line). The existing `forward_hotkey_signals` thread becomes the router's thread. A new `PipelineInput::Shutdown` ends a pipeline: it cancels a recording in progress, stops voice control listening and returns from `run` (closing the channel cannot end a pipeline that owns the spotter, which holds a sender). Unit-test swap, drop and shutdown.
4. **Shared overlay.** The overlay (`PointerSignIndicator` over `MacosVisualIndicator` and `MacosPointerSign`) is created once on the main thread and handed to every pipeline through a thin forwarding `VisualIndicator` that holds it behind an `Arc` and a lock; the pipeline keeps owning a `Box<dyn VisualIndicator>` as today.
5. **Runtime.** New module `app/src/runtime/` (a sibling of `adapters`, `config`, `core`; registered in `lib.rs`), the composition code moved out of `main.rs`: `wiring.rs` holds the former `connect_transcriber`, `connect_refiner`, `connect_command_spotter`, `connect_archive`, `chunking_cfg` returning errors instead of exiting; `mod.rs` holds `Runtime` with `start(config)` (the caller and the signals thread apply the overrides), `reload(config)`, `stop()`, `state()`, `observe(observer)`. The runtime runs its own thread (`runtime`) that owns the pipeline thread handle, the spotter and the current sender. The pipeline's parts come from a `PipelineAssembler` trait (the real one in `wiring.rs`; the component tests supply mocks). Reload sequence: state Restarting; build the new adapters from the new configuration on a new channel, probing the services while the dictation in progress goes on (a failure keeps the running pipeline untouched); wait until the activity is Idle; clear the router; send `Shutdown`; join the pipeline thread; spawn the new pipeline thread; set the router; state Ready. The hotkey binding is replaced before the new pipeline starts. CLI overrides (`--hotkey`, `--no-llm`, `--transcription-url`, `--refinement-url`, `--archive-dir`, `--no-archive`, `--voice-control`) are applied to every loaded configuration, on reload too, by `Cli::overlay`.
6. **Observers.** A `RuntimeObserver` trait (state_changed) in the runtime module; observers are stored in the runtime, called on the runtime thread; graphical observers of later changes dispatch to the main thread themselves. The log observer is the first one.
7. **SIGHUP.** `signal-hook` `=0.4.5` (pin exactly, AGENTS.md and the exact-versions rule) with `signal_hook::iterator::Signals` on a thread named `signals`; on SIGHUP: load and validate the configuration file, apply the CLI overrides, call `reload`; on an error log it and do nothing else. Only in the hotkey mode (not file mode).
8. **main.rs.** Keep: `Cli::parse`, config path, the setup wizard, load, overlay, validate, logging, file mode. Replace the rest with the runtime start, the signal thread, `NSApplication::sharedApplication(mtm).run()`. The `Preflight` call moves into `Runtime::start` for now (exit code 2 behaviour preserved); `permissions-gate-window` replaces it.
9. **Architecture test.** `app/tests/architecture.rs` gains: `src/core` never imports `crate::runtime`; `src/runtime` is the only place besides `main.rs` (file mode, one pipeline) that imports secondary adapters outside `src/adapters` or assembles a pipeline.
10. **Docs and glossary.** `docs/ubiquitous-language.md`: Runtime, Runtime state, Hotkey binding, Signal router, Pipeline activity; `docs/project-map.md`: the `runtime/` module; `docs/architecture/overview.md`: the runtime in the component view; `README.md`: a short "Reload the configuration without a restart" paragraph (`kill -HUP`).

Pitfalls: the pipeline's `run` loop returns only when every sender is dropped, so every clone of the input sender must be owned by something the runtime can drop or stop (the spotter, the router); the `MacosVisualIndicator` must stay on the main thread (it dispatches internally; keep it behind the forwarding wrapper, never recreate it); the hotkey listener thread must not hold a pipeline sender directly; a reload while the pipeline waits for focus (focus wait up to 300 s) counts as processing and waits too.

## Impact

- `app/src/main.rs` (shrinks), new `app/src/runtime/{mod.rs,wiring.rs,router.rs,observer.rs}`, `app/src/lib.rs`
- `app/src/core/application/hotkey_listener.rs` (binding parameter), `app/src/core/domain/hotkey.rs` (binding type), `app/src/adapters/primary/ptt/rdev/mod.rs` (classification function, binding read per event)
- `app/src/core/application/pipeline.rs` and `pipeline/` (activity register)
- `app/src/adapters/secondary/commandspotting/websocket/` (stop stays; no sender leak)
- `app/Cargo.toml` (`signal-hook = "=0.4.5"`), `Cargo.lock`
- `app/tests/architecture.rs`, new `app/tests/features/component/app_runtime.feature` and `app/tests/steps/app_runtime.rs`, unit tests in the files above
- `docs/ubiquitous-language.md`, `docs/project-map.md`, `docs/architecture/overview.md`, `README.md`
- Checks: `just test` after every edit; `just check` after the last task group; `just full-check` before the merge (build inputs change); manual run by the user (hotkey and recording area): dictate, edit the hotkey in `config.toml`, `kill -HUP`, dictate with the new key, paste the log.

## Open points

- Whether `SIGHUP` reload stays once the settings window exists: proposed yes, it costs nothing and helps development; remove it later if the user dislikes it.
- Where the restart waits when a dictation never ends (recording up to `max_recording_seconds`, focus wait up to 300 s): proposed to wait with no limit, with the state Restarting visible in the log; the settings window of a later change shows "applying after the current dictation".
- Whether the hotkey listener re-targeting needs a different design for chords (`Cmd+E`, `Option+E`): proposed the same binding read per event; the chord state machine resets when the binding changes.
- Module placement: `app/src/runtime/` as a top-level module beside `adapters`; alternative `app/src/app/`; proposed `runtime` because the glossary term is Runtime.

## Release

SemVer: MINOR — a new observable behaviour (reload without relaunch, SIGHUP) and a new capability; pre-1.0.
Target version: 0.28.0 (0.25.0, 0.26.0 and 0.27.0 were taken by sibling changes merged first; next above them; corrected at merge with `kit version-check` when a sibling change took it).
