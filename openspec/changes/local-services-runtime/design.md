# Design

## Context

The `justfile` supervises the services with bash (`supervise`, `start_service`, PID files, logs, 60 s readiness, health kill after 3 failures). The app only connects and exits 2 when a service is silent. `whisper-copy-pool` finds copies on convention ports, `restartable-app-runtime` gives the runtime states, `menu-bar-presence` the menu, `file-logging` the rotating writer and the logs folder.

## Decisions

**D1 — A use case with three ports, supervision as a tick-driven state machine.** `ServiceRuntime` in `core/application/local_services/` depends on the ports `ServiceLauncher` (spawn a `ServiceProcess`: exited?, terminate, kill), `ServiceProbe` (does the service answer) and `Clock`. Each supervised service is a `Supervision` whose `tick(now)` decides the next step (adopt, spawn, ready, health fail, restart pause, failed); one thread ticks every service every 100 ms. Unit tests drive `tick` with a fake clock, launcher and probe, so the whole schedule runs in milliseconds. Alternative: port the bash `supervise` as a script — rejected, no shell in a bundle and nothing testable.

**D2 — Adopt or own, decided once per service at start.** A port that answers as the expected service is adopted, never started over and never stopped; three failed checks make it Stopped outside the app, and it is adopted again when it answers. This keeps `just start-model-servers` useful in development and avoids port fights with the scripts' supervisor.

**D3 — The `justfile`'s numbers and arguments, guarded by a test.** Readiness 60 s, backoff 1 s doubling to 30 s (back to 1 s after a run of 30 s), 3 failed health checks every 5 s, engine arguments, thread split `cores / copies`. A unit test reads the `justfile`'s `whisper_args` and llama command lines and compares the flags, so the two cannot drift. Five starts in a row that never answered → Failed (the scripts stop at the first; the app retries a few times because a crash during the first Metal init is common).

**D4 — The copies stay found from the address.** The runtime starts the copies on exactly the ports `copy-pool` looks at (`ServiceAddress::copy_ports`), one copy on the service's own port; the pool keeps discovering them, so the recognition path does not change and no copy list is handed over.

**D5 — Services start at launch, beside the permissions wait.** Models take 5–20 s to load; the user grants permissions meanwhile. The runtime's `Supervisor` polls a `LocalServices` port (readiness of the services the configuration needs, `require`, `stop`) next to the permissions; the pipeline opens when both allow. A service Failed → runtime Failed without exiting, so Retry in the menu can bring it back. A reload calls `require` with the new configuration and waits for readiness before assembling. A service lost while Ready does not stop the pipeline: the failure notices of `keep-services-running` tell the user and the supervision restarts it.

**D6 — `[services] manage` as the switch.** False keeps today's behaviour exactly (probe at launch, exit 2); E2E and developers who prefer the `justfile` set it. Default true.

**D7 — Engines and models resolved in a fixed order.** Environment override, bundle `Contents/MacOS/engines/NAME/NAME`, `$PATH`, and for the command spotting service `target/{release,debug}/` next to the running binary. Models by the `justfile`'s file names in `[services] models_dir`, then `~/.voicepuncher/`; environment overrides win. A missing file → Failed with the path.

**D8 — Output piped into the rotating writer.** Both streams of a child go through two reader threads into one `RotatingFileWriter` per service (`BINARY-PORT.log`), so a full pipe never blocks an engine.

**D9 — Stop order and signals.** Command spotting first, then the whisper copies, then llama; SIGTERM to the pid, 5 s, SIGKILL. Quit stops through `Runtime::stop`; SIGTERM and SIGINT to the app stop the services too and end the process with code 0.

**D10 — Identity probes.** Whisper copies: `GET /health` 200 and `GET /` mentions Whisper; llama and command spotting: `GET /health` 200. An open port with another program is neither adopted nor started over: the service is Failed naming the port.

## Risks / Trade-offs

- [Two app instances] → the second adopts; the first one's quit stops them under it; a single-instance lock is a follow-up.
- [App killed with SIGKILL] → owned engines are orphaned; the next launch adopts them.
- [Bundled engines not there yet] → `$PATH` in development; `bundled-engines` fills the bundle folder.

## Test levels

| Scenario | Level |
|---|---|
| A cold launch starts the needed services | unit (`catalog`: needed services per configuration; `ServiceRuntime`) |
| Voice control needs the command services | unit (`catalog`) |
| A real launch leaves no service behind at quit | E2E `local_services.feature` (`@smoke`, test ports) |
| Services kept warm by the service scripts are adopted | unit (`supervision.rs`, `ServiceRuntime`: adopted, not stopped at stop); manual run |
| An owned service exits | unit (`supervision.rs`, fake clock) |
| The pause grows while a service keeps exiting | unit (`supervision.rs`) |
| An owned service that stops answering is restarted | unit (`supervision.rs`) |
| A service fails five starts | unit (`supervision.rs`) |
| Retry after a failure | unit (`supervision.rs`) |
| Quit with owned and adopted services | unit (`ServiceRuntime` stop order) |
| A service that ignores the request to end is killed | unit (`ServiceRuntime`, fake clock); adapter integration `localservices_process_it.rs` (terminate, kill) |
| The service scripts stop an adopted service | unit (`supervision.rs`) |
| A started service writes its own log | adapter integration `localservices_process_it.rs` |
| The app leaves the services alone | unit (`config.rs` default and `runtime/services.rs` `for_config`) |
| The refinement model is not on disk | unit (`catalog`) |
| The pipeline starts once the services are ready | component `app_runtime.feature` (fake services) |
| A service that cannot start fails the runtime without ending it | component `app_runtime.feature` |
| A retried service brings the runtime back | component `app_runtime.feature` |
| A service lost while Ready keeps the pipeline | component `app_runtime.feature` |
| Turning voice control on starts the command services | component `app_runtime.feature` |
| Stopping the runtime stops its services | component `app_runtime.feature` |
| Services still loading at launch | component `menu_bar_presence.feature` |
| The services are listed with their states | unit (`mapping` service lines); `menu_bar_presence.feature` |
| Restart from the menu | component `menu_bar_presence.feature` (actions over fake services) |

Arguments equal to the `justfile`'s, resolver order and model lookup: unit tests in `catalog`. Cold and warm launch with real engines, Restart from the real menu, an owned llama-server killed during use: manual run (deferred by the user for this run).
