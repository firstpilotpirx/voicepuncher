## 1. Domain and supervision

- [ ] 1.1 Red: unit tests in `app/src/core/application/local_services/supervision.rs` for adopt, spawn, ready, exit restart with 1/2/4 s pause, pause reset after 30 s, health kill after 3 failures, five failed starts → Failed, 60 s readiness, adopted gone and back, retry
- [ ] 1.2 Green: `app/src/core/domain/local_service.rs` (service kind, spec, state, ownership) and the ports `ServiceLauncher`, `ServiceProcess`, `ServiceProbe`, `Clock` in `app/src/core/application/local_services/`
- [ ] 1.3 Green: `Supervision::tick` in `supervision.rs`
- [ ] 1.4 Red: unit tests in `local_services/mod.rs` for `ServiceRuntime`: readiness count, stop order with kill after 5 s, adopted left running, missing program or model → Failed, restart of one service
- [ ] 1.5 Green: `ServiceRuntime` (ticking thread, snapshot, observer, restart, stop_all)
- [ ] 1.6 Add the glossary terms to `docs/ubiquitous-language.md`; `just test`; commit

## 2. Adapters and catalog

- [ ] 2.1 Red: `app/tests/localservices_process_it.rs` — spawn, terminate, kill, output in the rotating log file, probe against a listener and a health endpoint
- [ ] 2.2 Green: `app/src/adapters/secondary/localservices/process/mod.rs` and `probe/mod.rs`; `libc` as a dependency
- [ ] 2.3 Red: unit tests in `adapters/secondary/localservices/catalog/mod.rs` — whisper and llama arguments equal the `justfile`'s, copy ports, thread split, needed services per configuration, resolver order, model lookup, missing file
- [ ] 2.4 Green: the catalog
- [ ] 2.5 Red: unit tests in `app/src/config.rs` for `[services]` defaults and copies 1..=4
- [ ] 2.6 Green: `[services]` in `config.rs`; `just test`; commit

## 3. Runtime integration

- [ ] 3.1 Red: scenarios for the requirement "The runtime waits for the services it manages" in `app/tests/features/component/app_runtime.feature` with steps in `app/tests/steps/app_runtime.rs`
- [ ] 3.2 Green: `LocalServices` port in `app/src/runtime/services.rs`; `Supervisor` waits for services, reports Starting services, Failed without exit, retry, reload with `require`, stop stops services
- [ ] 3.3 Green: `RuntimeState::StartingServices` in `observer.rs`
- [ ] 3.4 Green: `main.rs` builds the managed services when `manage` is on (none in file mode), skips the launch probe of the command spotting service, stops services on SIGTERM and SIGINT in `signals.rs`; `just test`; commit

## 4. Menu

- [ ] 4.1 Red: scenarios for "The menu shows the services starting" and "The menu lists the services" in `menu_bar_presence.feature`, unit tests in `mapping/tests.rs`
- [ ] 4.2 Green: status line and icon for Starting services and service Failed in `mapping.rs`; service lines in words
- [ ] 4.3 Green: Services submenu with Restart and Open Log in `status_item.rs` and `actions.rs`; `just test`; commit

## 5. E2E and docs

- [ ] 5.1 Red: E2E scenario "A real launch leaves no service behind at quit" in `e2e-test/tests/features/local_services.feature` with steps
- [ ] 5.2 Green: make it pass on the test ports
- [ ] 5.3 Docs: `README.md` § Local services, `AGENTS.md` § What this project is, `docs/architecture/overview.md`, `docs/architecture/context.md`, `docs/project-map.md`; `just test`; commit

## 6. Release

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to 0.31.0; `just test`; commit
- [ ] 6.2 Manual checks by the user (deferred for this run by the user's decision): cold launch with no services, warm launch after `just start-model-servers` (adopted, left running at quit), quit leaves no owned process, kill an owned llama-server during use
