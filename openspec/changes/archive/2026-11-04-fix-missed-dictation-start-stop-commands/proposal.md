## Why

The voice commands that start and stop a dictation are periodically not recognized: the user says the start phrase or the stop phrase, and now and then nothing happens — no dictation starts, or the running dictation goes on. The failure is intermittent, not constant: the same phrases work most of the time, so the cause is not a missing phrase but something in the path from the microphone through the command spotting service (the Silero voice gate and the English and Russian sherpa-onnx streaming recognizers on port 8082) to the app's reaction to a reported command. A command that is missed at random makes hands-free use unreliable, and the user cannot tell whether the phrase was not heard, not spotted, spotted too late, or spotted and dropped by the app.

## What Changes

Settled scope (explore of 2026-11-04): the loss is inside the command spotting service, in two places; WebSocket delivery and the app lose nothing (every start, stop, enter, cancel and paste report of the current service build matches an app log line, and no report was dropped out of state).

1. **A one-word command settles only at the gate's pause.** The English recognizer hears the Russian "чтобы" as "Stop", and the decision stage reports "stop dictation" as soon as 0.2 s of audio is decoded past the word although the gate still hears speech (the next word "зайти" comes out of the recognizer 0.17 s later): the dictation ends with no text, and the user's real "стоп" after it is ignored in standby (command history entries `2026-11-04T13-58-55.846Z-86346-30006-684` and `…-687`, both with a start reported at 1.16 s and the same sentence repeated; the dictation archive holds two voice dictations with no text at those times). The specs already say a command said alone counts at the pause the gate hears after it; the code settles it on decoded time alone. The fix: mid-utterance, a one-word command counts only once the gate's pause after it began (within 0.35 s of the word, as today) and the recognizer decoded 0.2 s past the word; the close of the utterance and a pause the recognizer decoded up to settle it as before.
2. **A quiet word gets its tail before the final decode.** The user's quiet "старт" after a breath opens an utterance that the gate closes 0.4 s after the last speech it heard, and at that close neither recognizer has decoded the word: both final texts are empty (entries `2026-11-03T11-12-03.340Z-865-4-685` and `2026-11-02T11-07-47.633Z-45180-7594-910`, the gate's last speech at 0.97 s and 1.05 s, the close at 1.37 s and 1.46 s). The same audio followed by a second of silence is decoded by both recognizers ("стар", "Start"): a streaming recognizer emits a quiet word's last sounds only once it has heard the pause after them, and the 0.6 s of silence the service flushes at the gate's first quiet window came before the word's last sound. The fix: at the close of an utterance the service feeds every recognizer a tail of silence before their final decode, so a quiet word the gate stopped hearing early is still decoded at the close. A gate reset at the close was tried and refuted: a fresh detector hears the same windows.
- The user's recordings (sessions 1–5 in `~/.voicepuncher/voice-samples/`) do not reproduce either loss: their missed stops are the removed long form "stop dictation", their missed starts the tails of chains. Instead of a new recording session, the live misses are the recordings: the audio of the entries above with the speech before them (from the command history of the same connection) becomes recording session 6 (`session-6.wav`, `session-6/forms.jsonl`, in the form of the other sessions), and the reproducing tests run on it like on the others.
- Add a test that reproduces each loss and stays red until its fix (§ TDD bug-fix rule), at the level where the loss happens: the decision stage for 1 (its words and times taken from entry 684), a session with the real recognizers and the gate's hearing replayed from the command history for 2, and the streaming of session 6 in real time for 1.
- Measure on the user's recordings: `just streaming-bench --recorded` and the voice-sample tests on sessions 1–6 lose no command they recognize today, and the latency of "start" and "stop" said alone stays within its bounds.

Current stage: PoC. Target stage: PoC.
Scope: the recognition of the start and stop dictation commands inside the command spotting service — the decision stage and the final decode at the close of an utterance — and the recording session that reproduces their loss.
Acceptance bar: the reproducing tests that failed before the fix pass; the start phrases of session 6 are recognized and its "чтобы зайти в гетлаб" reports no stop; `just streaming-bench --recorded` and the voice-sample tests pass on sessions 1–6; `just check` passes; the user's manual run shows start and stop commands reacting every time over a session of repeated dictations.
Out of scope: the other voice commands, new command phrases or forms, changes to the dictation recognition model (whisper) or to the dictation text itself; the late reports of the English recognizer (about 0.6× real time, a start cue of 0.73 s at the 90th percentile), which lose no command; the bench's `--fast` mode, which reads the gate's state at the end of the clip and so cannot show loss 1 (a change of its own).

## Capabilities

### Modified Capabilities

- `voice-control`: a one-word command heard while the gate still hears speech is no command yet; a quiet command the gate stops hearing before the recognizers decoded it is still decoded at the close of its utterance.

## Impact

- `command-spotting/src/streaming/decide/` (the settling of a one-word command), `command-spotting/src/streaming/session/utterance.rs` (the tail of silence at the close) and `engine.rs` (a session with a gate of the test's choosing), their unit tests; the `needs_voice_samples` tests of `command-spotting/tests/it/streaming/` and `app/tests/it/voice_samples.rs` (session 6); `e2e-test/tests/features/voice_control.feature` for the new scenarios.
- On the user's machine, outside the repository: `~/.voicepuncher/voice-samples/session-6.wav` and `session-6/forms.jsonl`, assembled from the command history entries named above (the script and the entry names are recorded in the design).
- `just test`, `just check`, `just streaming-bench --recorded` on the user's recordings.

## Open points

All settled by the explore of 2026-11-04 (above): the stage is the command spotting service (the decision stage and the final decode at the close), the app and delivery are excluded by the evidence, and the live misses in the command history replace a new recording session. The target version 0.46.2 is a PATCH bump (0.46.1 was released by recognize-glued-enter-mishearings meanwhile); the chat names no version.

## Release

SemVer: PATCH — a bug fix in an existing capability, no new command or behavior.
Target version: 0.46.2.
