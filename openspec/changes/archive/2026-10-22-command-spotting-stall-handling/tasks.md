## 1. Room under the size budgets

- [x] 1.1 Move the unit tests of `command-spotting/src/{confirm,worker,outbox,protocol,server}.rs` to sibling `*/tests.rs` files
- [x] 1.2 Move the unit tests of `app/src/core/application/pipeline/voice_control.rs` and of the websocket `client.rs` to sibling `tests.rs` files
- [x] 1.3 `just test`; commit

## 2. A stalled copy hands over, a failed segment is reported

- [x] 2.1 Red: `confirm` tests — a copy past its decoding budget hands over to another copy; every copy stalled fails
- [x] 2.2 Green: `confirm.rs` retries a timed-out copy within the deadline
- [x] 2.3 Red: `command-spotting/tests/stall_it.rs` — a segment whose hearing fails yields `confirmation_failed`
- [x] 2.4 Green: `ServerEvent::ConfirmationFailed`, sent by the engine after a failed segment
- [x] 2.5 Red: `voice_control.feature` scenario "The app shows a failed confirmation" with its steps
- [x] 2.6 Green: client parses `confirmation_failed`; `SpottingEvent::ConfirmationFailed`; the pipeline shows the notice
- [x] 2.7 `just test`; commit

## 3. Stalls and bounded queues

- [x] 3.1 Red: `worker` tests — a job past the bound fails at once; the age of the running job is reported
- [x] 3.2 Green: bounded worker queues, logged send failures, running-job age
- [x] 3.3 Red: `health` tests — a hung confirmation makes the connection stuck
- [x] 3.4 Green: `Beat` takes the running-job age; `Health::stuck` counts it; the processing thread sends `stalled` once
- [x] 3.5 Red: `health` backlog tests — audio past 10 s is not admitted, admitted again once processed
- [x] 3.6 Green: the I/O thread's backlog and the `dropped` event
- [x] 3.7 Red: `app/tests/commandspotting_stall_it.rs` — `stalled` and `dropped` make the adapter lose and reconnect
- [x] 3.8 Green: the client turns `stalled` and `dropped` into connection errors
- [x] 3.9 `just test`; commit

## 4. Protocol version 2

- [x] 4.1 Red: `server_it.rs` — wrong version, malformed message and partial samples close with a reason
- [x] 4.2 Green: tagged `AppMessage`, `hello` first, close reasons, `samples_from_bytes` refuses partial samples
- [x] 4.3 Green: the app's client and every test client send `hello` and tagged messages
- [x] 4.4 `just test`; commit

## 5. Spotted checks and copy discovery

- [x] 5.1 Red: `engine/spotted.rs` test — `found_nothing` keeps a waiting check
- [x] 5.2 Green: `SpottedChecks` holds a list
- [x] 5.3 Red: `copy-pool` test — `discover` returns before a slow probe ends and the first lease waits for it
- [x] 5.4 Green: lazy first probe round; remove `is_same`
- [x] 5.5 `just test`; commit

## 6. Release

- [x] 6.1 Bump `version` in `app/Cargo.toml` to 0.37.0; add `confirmation failed`, `stalled`, `dropped` to `docs/ubiquitous-language.md`
- [x] 6.2 `just test`; commit
- [x] 6.3 Manual checks: none (no hotkey, recording, overlay layout, paste or permission change; the notice shown is an existing one)
