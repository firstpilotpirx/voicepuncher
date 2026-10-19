# Tasks

## 1. Count silence from the unanswered ping, after reading

- [x] 1.1 Red: in `app/tests/commandspotting_client_it.rs`, a stub service that answers pings; the client sends a ping, pauses 6 s, then reads — expect no error; run it and see it fail with "stopped answering"
- [x] 1.2 Red: same file, the client pauses 6 s right after connecting with no ping out, then pings and reads — expect no error; run it and see it fail
- [x] 1.3 `just test` (the two new tests red, the rest green) and commit `test: reproduce the false command spotting loss after a pause of the app (intentionally red)`
- [x] 1.4 Green: in `app/src/adapters/secondary/commandspotting/websocket/client.rs`, track the oldest unanswered ping, judge silence from it and only after reading, in `read_events` and `events_for_sent_audio`; `keep_alive` only pings, the first ping at once
- [x] 1.5 Keep `should_fail_the_wait_when_the_service_stops_answering` green within its bounds
- [x] 1.6 Update the adapter's call in `app/src/adapters/secondary/commandspotting/websocket/mod.rs` and the hang step in `e2e-test/tests/steps/local_services/hangs.rs` if their order no longer reads right
- [x] 1.7 `just test` and commit

## 2. Version and checks

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.21.1
- [x] 2.2 `just check` (runs `e2e-smoke` with the two long-listening tests on freshly started services) and commit
- [x] 2.3 Manual checks: none
