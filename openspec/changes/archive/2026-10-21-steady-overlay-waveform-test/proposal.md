## Why

The overlay integration test `should_show_the_record_sign_and_waveform_without_a_label` (`app/tests/visualfeedback_macos_it.rs:379`) fails now and then: its `assert_eq!` reads `0.0` where it expects `2.0`. It failed in `just test` on fix/speed-up-short-start-cue (2026-10-20 18:05) and in `just check` on fix/match-late-confirmation-to-its-state (2026-10-21 08:26). Both times other sessions ran checks on the machine. Neither branch touched the overlay. A flaky gate test blocks unrelated changes from landing.

## What Changes

- Find why the overlay sometimes reports `0.0` where the test expects `2.0`. Likely causes: the overlay is read before it is drawn, or the main thread is slow under machine load. Make the test deterministic without loosening what it proves.

Current stage: PoC. Target stage: PoC.
Scope: the overlay integration test and, if the cause is in the overlay, the overlay adapter.
Acceptance bar: the test passes in 20 consecutive `just test` runs while another checkout runs `just check`.
Out of scope: overlay behavior the user sees, unless the cause is a real overlay bug.

## Capabilities

### Modified Capabilities

- `visual-feedback`: none expected; only if the cause is a real overlay bug.

## Impact

- `app/tests/visualfeedback_macos_it.rs`, possibly `app/src/adapters/` overlay code.

## Open points

- Test bug or overlay bug: proposed choice, find out first; release only if overlay code changes.

## Release

SemVer: PATCH — a fix to a flaky test, or to the overlay if the cause is there.
Target version: 0.31.3.
