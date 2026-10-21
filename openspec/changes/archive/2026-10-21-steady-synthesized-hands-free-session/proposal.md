## Why

The smoke scenario "A hands-free session in a synthesized voice" fails often: the stage journal holds 17 failures since 2026-10-19 on more than ten branches. It is the most frequent reason a `just check` or `just full-check` fails and has to be run again, each time at the cost of a lock wait of up to an hour.

30 runs of the scenario alone reproduced it 5 times, all without extra load (none in 15 runs under full CPU load), so neither the services' speed nor the 30 s reaction wait is the cause. The failures have two causes, both real defects of voice control:

- In 16 of the 20 failures kept in session logs, the speech recognition service for commands heard the synthesized "cancel dictation" as "Capsule dictation" for one placement of the utterance in the stream; that is no command, so the dictation was not cancelled, and the next "start dictation please" was taken for a restart.
- In the other 4, the check that hears "start dictation" at the head of an utterance answered before the "please" after it; the recording began after "dictation", so "please." opened the pasted text.

## What Changes

- "Capsule dictation" counts as "cancel dictation", as the other slips of the speech recognition service already count as their commands.
- A "please" that opens a dictation started by "start dictation" or "restart dictation" is not pasted.

Current stage: PoC. Target stage: PoC.
Scope: the two causes above and the scenario's reliability.
Acceptance bar: the scenario passes 10 runs in a row on the test services; its expected sequence of app actions stays as it is.
Out of scope: the other hands-free session scenarios, the dictation path, other slips of the speech recognition service.

## Capabilities

### Modified Capabilities

- `voice-control`: one more slip of the speech recognition service is recognized as "cancel dictation"; a "please" after a voice start is not pasted.

## Impact

- `command-spotting/src/matcher.rs`
- `app/src/core/domain/command_words.rs`
- `app/tests/features/component/voice_control.feature`, `app/tests/steps/voice_control/speech_after_start.rs`

## Release

SemVer: PATCH — bug fix in voice control.
Target version: 0.31.1.
