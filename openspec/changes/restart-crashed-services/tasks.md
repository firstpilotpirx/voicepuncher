# Tasks

## 1. A stopped adopted copy is replaced; restarts ask the port first

- [ ] 1.1 Red: unit tests in `supervision_tests.rs` — an adopted service closed three checks in a row is started by the app and is owned; a due restart whose port answers Ready adopts it; a start that exits while its port answers Ready adopts it without counting a failed start (`just test-one`).
- [ ] 1.2 Red: rewrite the `Rule:` of the removed requirement in `app/tests/features/component/local_services.feature` into the rules "An adopted service that stops is replaced by the app's own" (three scenarios) and "A service that answers again before its restart is adopted", with their steps in `app/tests/steps/local_services*.rs` (`just test-one 'scenario: …'`).
- [ ] 1.3 Green: move the adopted watching into `supervision/adopted.rs`; closed three checks → one log line (label, port, reason) → decision; `Restarting` due → decision; exit with port Ready → adopt; remove `ServiceState::StoppedOutside` and its menu text (`app/src/core/domain/local_service.rs`, menu mapping tests).
- [ ] 1.4 `just static`, `just test`; commit `fix: replace an adopted service copy that stops`.

## 2. A hung adopted copy is stopped and replaced

- [ ] 2.1 Red: adapter integration test in `app/tests/it/` — the holder of a port whose command line names the service program is killed; another program is reported with its name and pid and left running (`just test-one`).
- [ ] 2.2 Green: make `holder_of` public in `copy-pool` (`copy-pool/src/probe/holder.rs`, `copy-pool/src/lib.rs`); add `ServiceLauncher::stop_holder` and its process adapter (`ps -o command=`, `SIGKILL`).
- [ ] 2.3 Red: unit tests — an adopted service busy three checks in a row has its holder stopped and is started again; a foreign holder is logged once and left (`just test-one`).
- [ ] 2.4 Red: component scenarios "A hung adopted copy is killed and replaced" (fake copy stopped with SIGSTOP) and "Another program on the port is not killed" under a new `Rule:` in `local_services.feature`, with steps.
- [ ] 2.5 Green: three busy checks of an adopted copy → `stop_holder` → log (label, port, pid) → decision; fakes in `fakes.rs` learn `stop_holder`.
- [ ] 2.6 `just static`, `just test`; commit `fix: replace a hung adopted service copy`.

## 3. End to end on real services

- [ ] 3.1 E2E scenario `@serial @resilience` "An adopted command spotting copy that stops is replaced" in `e2e-test/tests/features/local_services.feature` — the app adopts both command spotting copies, one copy's process is killed, within 30 s a copy answers on that port and a voice command works; steps in `e2e-test/tests/steps/`; its Red is the component scenario of 1.2, it runs in `e2e-resilience` of `full-check`.
- [ ] 3.2 `just static`, `just test`; commit `test: adopted command spotting copy replaced end to end`.

## 4. Version

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.41.2 (or the next version above `main`, `kit version-check`); `just test`; commit `chore: bump version to 0.41.2`.
- [ ] 4.2 `just check` passes once (`kit verify`); `full-check` with `e2e-resilience` runs in the landing group.
- [ ] 4.3 Manual checks by the user: none (no hotkey, recording, overlay, paste or permissions change).
