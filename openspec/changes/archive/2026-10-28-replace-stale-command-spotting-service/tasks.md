## 1. The greeting tells a copy's protocol version

- [x] 1.1 Red: unit tests in `app/src/adapters/secondary/commandspotting/websocket/client/` for the greeting against an in-test service that answers the hello (same version) and one that closes naming version 2 (other version, 2 read from the reason)
- [x] 1.2 Green: the greeting function (hello + ping, 2 s wait) in the command spotting client
- [x] 1.3 `just test` and commit

## 2. Voice control passes over a copy of another version

- [x] 2.1 Red: scenario "One of two copies speaks an older protocol version" under a new `Rule: Voice control passes over a copy of another protocol version` in `app/tests/features/component/local_services.feature`, with steps in `app/tests/steps/local_services/`
- [x] 2.2 Green: the client's connection to the copies greets each candidate and skips one refusing for its version, reporting the refused port to an optional hook
- [x] 2.3 The stand-in services of the client tests take the greeting before they speak (`test_stream::take_greeting`)
- [x] 2.4 `just test` and commit

## 3. Adoption checks the protocol and replaces a stale copy

- [x] 3.1 Red: supervision unit tests in `app/src/core/application/local_services/`: another version at the decision has its holder stopped and the app's own copy started, with one log line; the same version is adopted; a foreign holder fails the service naming the port
- [x] 3.2 Green: `ServiceProbe::protocol` with its fake in `fakes.rs`, and the protocol branch of `Supervision::decide` calling `stop_holder`
- [x] 3.3 Green: `HttpProbe::protocol` in `app/src/adapters/secondary/localservices/process/probe.rs` through the greeting
- [x] 3.4 Red: scenarios "A copy left by an older app is replaced at launch", "A copy of the same protocol version is adopted as before" and "Another program refusing the app's protocol is not stopped" under a new `Rule: A command spotting copy of another protocol version is replaced` in `local_services.feature`
- [x] 3.5 Green: steps for them in `app/tests/steps/local_services/`
- [x] 3.6 `just test` and commit

## 4. A refusal while voice control runs rechecks the copy

- [x] 4.1 Red: scenario "A stale copy appears after the app started" under a new `Rule: A protocol refusal while voice control runs replaces the copy` in `local_services.feature`, and a unit test that `Services::recheck` sends an adopted unit back to Pending
- [x] 4.2 Green: `recheck` on `Services`, `ServiceRuntime` and `ServicesView`; the command spotting adapter's refused-port hook wired to it through the `WiringAssembler` in `app/src/menubar_app.rs` when the app manages its services
- [x] 4.3 `just test` and commit

## 5. Release

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.41.3
- [x] 5.2 `just test`, commit, then `just check` once
- [x] 5.3 Manual checks: none (no hotkey, recording, overlay, paste or permissions change)
