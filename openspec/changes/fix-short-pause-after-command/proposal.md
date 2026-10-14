# Proposal

## Why

A command said on its own, with speech after a short pause, often does not take effect, although the spec says an utterance ends after 0.3 s of silence. Three cases turned up while testing `keep-speech-after-start` (v0.10.1):

- **The user's quiet command runs into the next speech.** The user's recorded "restart" (`~/.voicepuncher/voice-samples/session-2.wav`, 27.23–27.52 s), followed by 0.4–0.6 s of silence and then speech, became one utterance. The voice activity detector did not end it, so the lone "restart" was no command.
- **The command check hears the next word.** With a 0.4 s pause, the clip the service sends to confirm a command runs 0.2 s past a segment end the detector places late. It can take in the first sound of the next word. A synthesized "Restart", 0.4 s, then "Coffee…" was heard as "Restocked", and no command took effect.
- **The tests do not measure the pause they name.** macOS `say` ends every word with about 0.4 s of its own silence. So the E2E scenario "Stop after a short breath" (0.35 s in the step) really pauses about 0.75 s, and the 0.3 s rule goes untested.

The user dictates hands-free and does not wait a full second after "start", "stop" or "restart". A command that silently fails breaks the flow.

## What Changes

- A command said on its own, followed by a pause of at least 0.3 s and then speech, takes effect. This holds in the user's own recorded voice and in synthesized speech.
- The clip that confirms a command holds only the command, never the first sound of the speech after it.
- The E2E scenarios about pauses measure the real pause: the silence `say` adds after a word is not counted as part of the pause.
- No change to which phrases count as commands, to commands said without a pause, or to dictation.

## Capabilities

### Modified Capabilities

- `voice-control`: "An utterance ends after 0.3 s of silence" holds for a quiet voice and when speech follows the pause at once. Its scenarios gain the user's recorded commands followed by speech after a short pause.

## Impact

- The command spotting service (`command-spotting/src/engine.rs`): the end of a segment and the clip sent for confirmation.
- E2E steps that build pauses from `say` speech (`e2e-test/tests/steps/voice_control.rs`).
- Tests on the user's recordings (`~/.voicepuncher/voice-samples/`): "restart", "start", "stop" and "enter" alone, each followed by speech after 0.3–0.6 s. A new recording session may be needed when the existing labels do not cover them.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:**
  - A command alone, then 0.3–0.6 s of silence, then speech takes effect for every command form in the user's recordings and in synthesized speech.
  - No command takes effect for the existing negative cases (a command word inside a sentence, short Russian phrases, corpus speech).
  - `just full-check` shows no new failures.
- **Out of scope:**
  - Commands said without a pause; they are handled already.
  - A different keyword spotting or voice activity model, unless measurements show the current one cannot meet the bar.
  - The dictation's own transcription.

## Release

PATCH: a bug fix. The target is the next patch after the version on `main` when this lands.
