## Why

Two long-listening tests in `app/tests/commandspotting_websocket_it.rs` (`should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them` and `should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening`) failed twice on 2026-10-18, during the checks of show-live-transcript. Both times it was at the start of `e2e-smoke`, about 47 s in, right after the check started its own services on ports 28081/28085/28082. They failed with `unexpected input SpottingLost` at `app/tests/commandspotting_websocket_it.rs:276`. The command spotting service log showed no error on its side: the app was the one closing each connection. Each failure passed on one unchanged rerun. The same test failing again counts as a repeated flake, and the project rule says to file its cause.

So the app's command spotting adapter reports the service lost while the service is alive and freshly started. In daily use the same false loss would make voice control drop out for no reason.

## What Changes

- Voice control no longer reports the command spotting service lost while that service is up and answering, including right after it starts and while several connections stream audio at once.
- The two long-listening tests pass on freshly started services, every run.

Current stage: PoC. Target stage: PoC.
Scope: stop the command spotting adapter from counting a pause of its own as the service's silence (§ Cause).
Acceptance bar: the cause is named, a test reproduces it red, and after the fix the two tests pass in `e2e-smoke` on freshly started services across repeated runs.
Out of scope: loosening the test bar, raising time limits to hide the loss, and the outage handling for a service that is really down (show-spotting-away).

## Capabilities

### Modified Capabilities

- `voice-control`: a live command spotting service is never reported lost.

## Impact

- `app/src/adapters/secondary/commandspotting/websocket/` (outage detection), possibly `command-spotting/` (health, warm-up of the speech recognition service for commands)
- `app/tests/commandspotting_websocket_it.rs`
- `just check`, `just full-check` (`e2e-smoke`)

## Cause

Found in explore and reproduced: under CPU load the two tests fail with `SpottingLost` about 5.2 s in, in 4 of 6 runs; without load in none of 3. The service answers every ping on its own I/O thread, on time. The app's streaming worker is the one that goes quiet: the test hands it 30 s of audio at once, and resampling that in a debug build takes about 1.2 s on an idle machine and several times longer under load. While it resamples it reads nothing, so the pongs wait unread in the socket. Then it checks "has the service been silent for more than 5 s?" before reading them, and declares the service lost. Any pause of the app's own longer than 5 s (a stalled worker, a sleeping laptop) does the same in daily use.

## Decisions taken in explore

- The fix: the app reads what the service already sent before it judges silence, and counts silence from the oldest ping still unanswered, not from the last answer it happened to read. A pause of the app's own never counts as the service's silence.
- The red test: a client test against a small server that answers pings, with the app paused for 6 s; today the connection is reported lost.
- Capability `voice-control`, requirement "A slow command spotting service does not stop listening", modified.
- Target version 0.17.1, a PATCH above 0.17.0.
- Out of scope: making the resampling of a large burst of audio cheaper; it shortens the pause but does not remove the false loss.

## Release

SemVer: PATCH — bug fix.
Target version: 0.17.1.
