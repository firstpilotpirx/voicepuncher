# Design

## Context

The `justfile` supervises the services with bash (`supervise`, `start_service`, PID files, logs, 60 s readiness, health kill after 3 failures). The app connects and exits 2 when a service is silent. `whisper-copy-pool` gives the copy ports and a pool that can take an explicit copy list. `restartable-app-runtime` gives states and the pipeline start; `menu-bar-presence` the menu; `file-logging` the rotating writer and the logs folder. The user wants a separate component that owns models, services and routing, and warm services across app restarts in development.

## Decisions

**D1 — A use case with three ports, supervision as a clock-driven state machine.** `ServiceRuntime` in `core/application/local_services/` depends on `ServiceLauncher`, `ServiceProbe` and `Clock`; the supervisor loop per service is a pure state machine stepped by the clock, so the whole restart and health schedule is unit-tested with fakes in milliseconds. The process adapter is thin. Alternative: port the bash `supervise` as a spawned shell script — rejected, no shell in a bundle and nothing testable.

**D2 — Adopt or own, decided once per service at start.** A port that answers with the expected service is adopted; the app never starts a second copy on a port it did not own and never stops an adopted one. This keeps `just start-model-servers` useful in development (models stay loaded across `cargo watch` restarts) and prevents port fights with the scripts' supervisor. An adopted service that disappears is reported and waited for (D6 of `menu-bar-presence` shows it).

**D3 — The `justfile`'s numbers and arguments, verified by a test.** Readiness 60 s, backoff 1 s doubling to 30 s, 3 failed health checks, and the engine arguments are copied from the `justfile`; a unit test parses the `justfile`'s `whisper_args` and llama lines and compares, so the two cannot drift until a later change makes the scripts call the app.

**D4 — Copy list handed to the pool.** The runtime knows the ports it started or adopted and gives them to `CopyPool::with_copies`; discovery by probing stays the fallback for `manage = false`.

**D5 — `[services] manage` as the switch for tests and the scripts.** E2E and developers who prefer the `justfile` set it to false and get today's behaviour; the default is true for users.

**D6 — Engines resolved by a fixed order, models by the `justfile`'s file names.** Environment override, bundle folder, `$PATH`, `target/` — no config key for binaries; model file names are the ones every recipe and README already use, in `[services] models_dir`.

**D7 — Stop order and signals.** Command spotting first (it holds connections to the commands copies), then whisper copies, then llama; SIGTERM, 5 s, SIGKILL, to the pid only.

**D8 — Failures do not stop the pipeline once Ready.** A service lost while dictating is already handled by the failure notices (`keep-services-running`); the runtime restarts it and the next dictation works; only the start waits for readiness.

## Risks / Trade-offs

- [Adopting the wrong process on a port] → the probe checks the service's own answer (`/health` JSON, whisper's page), not only the open port.
- [Two app instances] → the second adopts; documented; a single-instance lock is a follow-up.
- [The bundle's engines are not there yet] → `$PATH` resolution works in development; `bundled-engines` adds the bundle folder.
- [Engines write a lot to their logs] → rotated by size like the app log.

## Test levels

| Scenario | Level |
|---|---|
| Restart schedule, health kill, five failures → Failed, adopt decision, stop order, readiness count | unit tests in `local_services/supervisor.rs` (fake clock, launcher, probe) |
| Arguments equal the `justfile`'s; resolver order; model paths | unit tests in `catalog.rs` |
| The app starts the services it needs and dictation starts when all are ready | component `local_services.feature` (mock launcher/probe, runtime) |
| A service that already answers is adopted and left running at quit | component `local_services.feature` |
| An owned service that stops is started again; cannot start → Failed with Retry | component `local_services.feature` |
| Quit stops every owned service; adopted gone is reported and waited for | component `local_services.feature` |
| Services shown in the menu; per-service log files; `manage = false` | component `local_services.feature` (menu snapshot mapping), unit |
| Spawn, terminate, kill, log file; probe against a listener | adapter integration `localservices_process_it.rs` |
| A real cold start on the test ports: processes exist, quit leaves none | E2E `local_services.feature` (`@smoke`, own ports) |
| Cold and warm start, kill llama-server, quit | manual run by the user |
