# Design

## Context

`main.rs` is the composition root and the only owner of the pipeline thread, the hotkey listener thread, the overlay and the command spotting client. The GUI changes need to stop and restart the pipeline in place. `rdev::listen` never returns and has no stop call. The overlay holds AppKit objects created on the main thread. `Pipeline::run(rx)` ends when every `PipelineInput` sender is dropped. `WebSocketCommandSpotter::new(url, inputs_tx)` holds a sender and `stop(self)` ends it.

## Decisions

**D1 — A runtime module outside `core`, beside `adapters`.** `app/src/runtime/` is composition code (it imports adapters and application ports), so it lives outside `core` where the architecture test forbids adapter imports, and outside `adapters` where every module must be a `feature/tech` adapter. `main.rs` calls only `runtime::Runtime` and file mode. Alternative: keep everything in `main.rs` behind functions — rejected, the GUI adapters need to call reload and observe state, and `main.rs` is not a library module.

**D2 — The hotkey listener is started once and re-targeted; the pipeline is replaced.** Because rdev cannot stop, the listener thread lives as long as the process and reads the current hotkey binding on every event (a lock around `HotKey`; one read per key event is cheap). The pipeline, the recorder, the transcriber, the refiner, the paster, the sound player and the spotter are rebuilt on every reload from the new configuration — simpler and safer than making each adapter reconfigurable. Alternative: a stoppable listener via CGEventTap directly — rejected for this change, it would replace the whole adapter.

**D3 — Signals reach the pipeline through a router the runtime swaps.** The router holds `Option<Sender<PipelineInput>>` behind a mutex; the runtime sets it before a pipeline starts and clears it before stopping one. Dropping the last sender ends `Pipeline::run`, which is the stop mechanism: no new input variant, no change to the pipeline's state machine. The spotter is stopped first because it holds a sender clone.

**D4 — Restart waits for Idle.** The pipeline publishes its activity (Idle, Recording, Processing, the latter including the focus wait) in an atomic register; `reload` waits for Idle before closing the channel, so a dictation is never cut. The wait has no limit (open point in the proposal); the state Restarting is observable meanwhile.

**D5 — One overlay for the life of the process.** The macOS indicator and pointer sign are created once in `main` with the main thread marker and shared through a forwarding `VisualIndicator` (`Arc<Mutex<Box<dyn VisualIndicator>>>`); each pipeline gets a new forwarding box. The overlay already dispatches its drawing to the main thread, so the forwarding box only needs `Send`.

**D6 — Observable state through a trait, called on the runtime thread.** `RuntimeState` is an enum (Starting, Ready { voice_control }, Restarting, Stopped, Failed(String)); `RuntimeObserver::state_changed(&self, &RuntimeState)` is called synchronously on the runtime thread, with the current state at registration. GUI observers dispatch to the main thread themselves. Alternative: a broadcast channel — rejected, every consumer would need its own thread.

**D7 — SIGHUP through `signal-hook` 0.4.5.** A `signals` thread iterates `Signals::new([SIGHUP])` and calls reload with the freshly loaded, overlaid and validated configuration. The pinned exact version follows the exact-versions rule. Alternative: `libc::signal` with a static flag — rejected, signal-safe handling is what the crate is for.

**D8 — Failure keeps today's exit code.** `Runtime::start` returns an error when a service does not answer or a permission is missing (through `Preflight`), and `main` exits with code 2 as today. The state Failed carries the reason for observers of later changes.

## Risks / Trade-offs

- [A reload rebuilds the transcriber, which probes the service and may take up to its connect timeout] → the state Restarting is visible; the probe is the same as at start.
- [Rebuilding the recorder reopens the microphone; macOS shows the orange microphone dot again] → the dot is shown only while a recording runs (cpal opens the stream per recording), unchanged.
- [The rdev thread holds the hotkey lock per event; a reload that changes the binding while a chord is half pressed] → the chord state machine resets on a binding change (D2).
- [Two restarts requested at once (SIGHUP twice)] → reload calls are serialised on the runtime thread; the second one sees the new configuration and restarts again.

## Test levels

| Scenario | Level |
|---|---|
| The pipeline starts from the configuration; a silent service makes the state Failed | component `app_runtime.feature` (mock transcriber probe) |
| A new configuration applies without a relaunch (new hotkey records, old does not) | component `app_runtime.feature` (fake listener feeding the binding) |
| A restart waits for the dictation in progress | component `app_runtime.feature` (activity register driven by the test) |
| A rejected configuration keeps the running pipeline | component `app_runtime.feature` |
| The runtime state is observable, in order, including the state at registration | unit tests in `runtime/mod.rs` |
| SIGHUP reloads the configuration | adapter integration `runtime_signal_it.rs` (sends SIGHUP to itself with a temp config) |
| Stopping the runtime ends the pipeline and the spotter | component `app_runtime.feature` |
| Event classification against a binding (single key, chord, binding change mid-chord) | unit tests in `adapters/primary/ptt/rdev/` |
| Router swap and drop | unit tests in `runtime/router.rs` |
| `core` never imports `runtime` | `app/tests/architecture.rs` |
| A real dictation after a `SIGHUP` hotkey change | manual run by the user |
