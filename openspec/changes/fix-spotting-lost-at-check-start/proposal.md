## Why

Two long-listening tests in `app/tests/commandspotting_websocket_it.rs` (`should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them` and `should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening`) failed twice on 2026-10-18, during the checks of show-live-transcript. Both times it was at the start of `e2e-smoke`, about 47 s in, right after the check started its own services on ports 28081/28085/28082. They failed with `unexpected input SpottingLost` at `app/tests/commandspotting_websocket_it.rs:276`. The command spotting service log showed no error on its side: the app was the one closing each connection. Each failure passed on one unchanged rerun. The same test failing again counts as a repeated flake, and the project rule says to file its cause.

So the app's command spotting adapter reports the service lost while the service is alive and freshly started. In daily use the same false loss would make voice control drop out for no reason.

## What Changes

- Voice control no longer reports the command spotting service lost while that service is up and answering, including right after it starts and while several connections stream audio at once.
- The two long-listening tests pass on freshly started services, every run.

Current stage: PoC. Target stage: PoC.
Scope: find why the command spotting adapter reports `SpottingLost` against a live, freshly started service, and fix that cause.
Acceptance bar: the cause is named, a test reproduces it red, and after the fix the two tests pass in `e2e-smoke` on freshly started services across repeated runs.
Out of scope: loosening the test bar, raising time limits to hide the loss, and the outage handling for a service that is really down (show-spotting-away).

## Capabilities

### Modified Capabilities

- `voice-control`: a live command spotting service is never reported lost.

## Impact

- `app/src/adapters/secondary/commandspotting/websocket/` (outage detection), possibly `command-spotting/` (health, warm-up of the speech recognition service for commands)
- `app/tests/commandspotting_websocket_it.rs`
- `just check`, `just full-check` (`e2e-smoke`)

## Open points

- The cause is not known yet. The candidates are the adapter's loss timeout under the load of parallel tests, and the speech recognition service for commands still warming up. Proposed: measure both in design before choosing a fix.
- Capability `voice-control`: proposed as the capability that holds the loss behavior. If another spec owns it, use that one.
- Target version 0.17.1: proposed as a PATCH above 0.17.0.

## Release

SemVer: PATCH — bug fix.
Target version: 0.17.1.
