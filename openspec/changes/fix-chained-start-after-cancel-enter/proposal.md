## Why

`app/tests/voice_forms_it.rs` is red on `main` (d15245a, after `recognize-users-quiet-restart` landed). One recorded form of the user goes wrong:

- session-4 at 32.41 s, while dictating, the user says "never mind cancel enter start" (a chain of three commands). The app carries out "cancel dictation" and "press enter" and loses "start dictation"; expected all three, in order.
- The command spotting service's log shows the speech recognition service for commands (whisper-server with `ggml-small.bin`) hearing the utterance as " Never mind, cancel dictation." for both its window and its segment confirmations, so the "enter start" end of the chain is never heard as such.
- It fails 4 out of 4 runs with `main`'s command spotting service and 4 out of 4 with the build of `detect-quiet-clear-all-text`, all services on ports of their own (speech recognition 9381, refinement 9380, command spotting 9382, speech recognition for commands 9385), so it is not a flake.
- With the voice activity detector input raised × 2 (a variant tried in `detect-quiet-clear-all-text`) this form passed once, which hints the result depends on where the utterance's segment and clip start and end.

This breaks the `voice-control` promise that every command the user said in a recording kept for testing is recognized exactly once, makes `just check` and `just full-check` red on every branch, and blocks `detect-quiet-clear-all-text` from landing.

## What Changes

- The user's recorded "never mind cancel enter start" during a dictation carries out "cancel dictation", "press enter" and "start dictation", in that order, with no other command.
- Find out why the speech recognition service for commands hears only "Never mind, cancel dictation." — which clip it gets (segment bounds, pre-roll, window), and whether the second hearing of `recognize-users-quiet-restart` or the separate commands model plays a part — and fix that without changing what is heard for the other recorded forms.
- `voice_forms_it` passes again on `main`; nothing is added to its `KNOWN_WRONG` list.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: the requirement already exists (`voice-control`, "The user's own way of saying the commands is recognized").

## Scope

- Current stage: PoC. Target stage: PoC.
- Acceptance bar: `voice_forms_it` passes on ports of its own with every recorded form, the session-4 32.41 s chain included; `voice_samples_it` and every voice control E2E scenario still pass; `just full-check` shows no new failures.

## Out of scope

- A different speech recognition model for commands, or new command phrases.

## Release

PATCH (bug fix): 0.15.2, or the next patch above `main` when it lands (`detect-quiet-clear-all-text` waits for this change).

## Open points

- Whether `main` turned red with `recognize-users-quiet-restart` (its second hearing of a near miss of "restart"), with the separate speech recognition service for commands, or earlier; bisect before fixing.
- Whether this case passes under the shared services of `just check` (ports 8081/8082/8085) and only shows up on ports of its own.
