# Proposal

## Why

The user says "start start" and goes straight on with the dictation, and the first second or so of it is missing from the pasted text. In the command spotting log, "Start start 1, 2, 3, 4" was reported as a doubled command (`Anywhere`) about 2 s into the utterance, and the dictation held only what came after. Only a "start" that opens an utterance (`Head`) gives the dictation the speech heard after it. Every other start ("start" alone followed by a short pause, "start start", a chain ending in "start", "restart") opens the microphone only once the command is reported. So everything said between the end of the command and that moment is lost. Even for `Head`, the heard speech is taken before the microphone opens, so the time it takes to open is lost too.

## What Changes

- A dictation started by voice begins where the command's words end, not where the app reacted to it. The speech heard since the command ended comes from the audio the app hears all the time for command spotting: the sound itself (16 kHz mono, as the speech recognition service takes it), never a transcript of it, so the whole dictation is transcribed in one go. It is joined to the recording with no gap and nothing heard twice. This holds for every placement of "start" (alone, at the head, doubled, at the end of a chain) and for "restart".
- The command words stay out of the pasted text, also when a doubled "start start" reaches the transcript.
- No change to the hotkey, to which phrases count as commands, or to how a dictation ends.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: a dictation started by voice holds all the speech said after the command, the first word included, however "start" or "restart" was said.

## Impact

- `app/src/core/application/pipeline/voice_control.rs` (every start and restart begins with the heard speech), `app/src/core/application/pipeline.rs`, the `AudioRecorder` port and its cpal adapter (join the heard speech at the recorder's own start), the `CommandSpotter` port and its WebSocket adapter (heard audio between two moments), `app/src/core/domain/command_words.rs` (the opening "start" or "restart", also doubled).
- Tests: unit tests for the join and the placements, `voice_control` component and E2E scenarios with synthesized speech, and tests on the user's own recorded "start start" and "restart restart" (`~/.voicepuncher/voice-samples/`), each followed at once by a sentence.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** "start start" followed at once by a sentence pastes the whole sentence without "start". "start", a pause of about 0.5 s, then a sentence pastes the whole sentence. "restart", a pause of about 0.5 s, then a sentence pastes only that whole sentence. Each holds in synthesized speech; "start start" and "restart restart" followed at once by a sentence also hold in the user's own recorded voice. `just full-check` passes, and the user's manual run shows the first word kept.
- **Out of scope:**
  - Reporting "start start" sooner, so the overlay appears earlier.
  - Recording the whole dictation from the command spotting stream instead of the recorder's own.
  - The hotkey's recording start.
  - The speech recognition service's latency.

## Release

PATCH: a bug fix. Target version **0.10.1**, or the next patch if another fix releases first.
