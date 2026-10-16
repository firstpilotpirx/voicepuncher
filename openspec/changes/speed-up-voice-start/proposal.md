# Proposal

## Why

The user, translated from Russian: "When I say, for example, 'start start', the recorder starts, but not right away — about one and a half, even two seconds later. Need to understand why, and fix it so it starts as fast as possible."

Hands-free, the user waits for the start sound and the overlay before speaking, so every voice start costs that wait, and a silence of 1.5–2 s after a command feels broken. The spec already promises that a doubled short form is carried out "at once". A start is the voice command the user says most: 12 of 24 in one hour of use on 2026-10-16.

## What Changes

- **Measure first.** Before any fix, time each stage between the end of the command's words and the start cue on the developer machine, on the user's recorded commands and in live use. The causes below are estimates from the code and the service log; the measurement replaces them in `design.md` and picks the fixes.
- **A timing log for every voice start**, like `stop→paste`: one line with the time from the end of the command's words to the start cue and the share of each stage. It stays right after microphone audio was lost: the app dates the service's events from the capture start, and on 2026-10-16 the service's stream time fell ≈8 s behind the clock at about 14:30 UTC and stayed there, as about 8 s of microphone audio never reached it.
- **The start cue follows the command as soon as the command is confirmed**, for every voice start: "start dictation" or "start" said alone, opening or ending an utterance, doubled, or last in a chain from standby, and "restart" in each of its forms. The start cue is the start sound, the overlay's recording sign and the recording running. What the dictation holds stays as it is: it begins where the command's words end.
- **Commands stay as reliable as today**: command accuracy and the protections against false starts (a command phrase inside a sentence, ordinary speech, short Russian phrases, "start stop") do not get worse.

### Known and suspected causes

Investigation of 2026-10-16: the code, and the command spotting service log of the user's session. Developer machine: Apple M1 Max, `ggml-small`, three speech recognition copies.

| # | Wait | Starts it delays | Estimate |
|---|---|---|---|
| 1 | **Timed checks of a running utterance** (command spotting service). A start followed at once by speech is found only by a check of the utterance while it runs: its head ≈1.2 s after the voice activity detector noticed speech (up to 0.7 s after the speech began), its last 2.5 s first 2.0 s after that notice, then every 1.0 s; a check that falls due while another waits is skipped. | "start" opening an utterance, "start start", a chain ending in "start", "restart restart", each followed at once by speech | ≈1.0–2.0 s |
| 2 | **The head check ignores a doubled start** (command spotting service). It takes only "start" or "start dictation", alone so far or followed by other words; "start start" and chains count only from the next check. In the log, head checks heard " Start start" and " Start, start, dime.", and the dictation started only with the check ≈0.8 s later. The archived `keep-speech-after-start` saw "Start start 1, 2, 3, 4" reported ≈2 s into the utterance and left reporting it sooner out of scope. | "start start" opening an utterance | ≈0.8 s of row 1 |
| 3 | **The utterance must end first** (command spotting service). A start said on its own is confirmed only after 0.28 s of quiet ends its utterance, or up to ≈0.85 s when the detector ends it itself (it holds quiet speech up to 0.55 s into the pause). | a start followed by a pause | ≈0.3–0.85 s |
| 4 | **The confirmation** by the speech recognition service. In the user's session ≈110–360 ms (closed utterances: median 171 ms, p90 212 ms); whisper.cpp encodes a full 30 s window even for a 1–3 s clip. A connection's confirmations run one at a time, so one waits behind a check in flight: once behind a check that took 1.9 s repeating a word up to its limit. | every start | ≈0.17 s, rarely up to ≈2 s |
| 5 | **"restart" waits for the dropped dictation** (app). Cancelling sends the rest of the dictation (up to 30 s) to live transcription and waits for its text, then drops it; only then do the cancel and start sounds play. | "restart" in every form | ≈0.3–1.5 s, longer for a longer dictation (estimate) |
| 6 | **Starting the cue** (app). Each sound is a new `afplay` process; the overlay is shown only after the recorder has opened a second CoreAudio input stream and joined the lead-in. | every start, the hotkey's too | ≈50–200 ms (estimate) |
| 7 | **Transport** (app and service): the microphone buffer and the 1024-frame resampler chunk (≈21 ms at 48 kHz), 10 ms socket read waits on both sides, the service's 20 ms poll for answers. | every start | ≈20–60 ms |

Today's total, by estimate: ≈0.6 s (up to ≈1.4 s) for a start followed by a pause; ≈1.4–2.4 s for a start followed at once by speech, the user's case. No log measures any of it: the app times only `stop→paste`, and the service logs each confirmation's time with 1-second timestamps.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: every voice start gives its start cue within a stated time of the end of the command's words (the "at once" of a doubled short form gets that bound), and logs that time by stage.

`local-services` keeps its rules for the command spotting service (at most one check of a running utterance waiting; a closed utterance's confirmation tried twice). No other capability changes.

## Impact

- Command spotting service: `command-spotting/src/engine.rs` (when a running utterance is checked and when it ends), `engine/confirming.rs` and `matcher/placed.rs` (what each check accepts), `engine/pause.rs`, `worker.rs` (one confirmation thread per connection); `protocol.rs` and `outbox.rs` only if the timing travels with the events.
- App: `app/src/core/application/pipeline.rs` (`start_recording_from`, `cancel_recording`), `pipeline/voice_control.rs`, the command spotting WebSocket adapter, the macOS sound player, the cpal recorder's start, the macOS overlay. The hotkey shares the recording start, so it may start faster too.
- Tests: a failing reproduction first ("start start" followed at once by a sentence is reported long after its words); when a start is reported in the command spotting tests (`engine_it`, `placed_commands_it`); `voice_control` component and E2E scenarios; the user's recorded commands (`~/.voicepuncher/voice-samples/`). `command-spotting/src/*` is on `recognition_paths`, so `just full-check` runs 100 accuracy recordings.
- Docs: `docs/ubiquitous-language.md` (*start cue*, if the specs use the term); AGENTS.md § How to verify a change and README § Release checklist (the timing line in a healthy log).
- Same code as `recognize-users-quiet-restart` (the confirmation clip), `restore-recognition-prompt` (the confirmation request and the whisper-server flags, so the confirmation time), and `confirm-voice-commands`, `show-dictation-cursor` and `notify-service-failures` (cues of voice commands and of the recording).
- Manual run by the user: the recording start, the overlay and the sounds change.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **In scope:** the time from the end of a voice start's words to its start cue, for every voice start from standby and for "restart"; its measurement and its log.
- **Acceptance bar:**
  - On the developer machine, with the services otherwise idle, measured by the new timing log over the user's recorded commands and synthesized speech for each placement: every voice start in scope gives its start cue within **700 ms** of the end of the command's words in at least 9 of 10 starts. The design aims lower and states the floor of each path: a start followed by a pause cannot beat the 0.3 s of silence that ends its utterance plus one confirmation (≈0.17 s) unless the confirmation overlaps that silence.
  - The user's manual run shows the timing line within the bar for "start start" followed at once by speech and for a start said on its own.
  - Nothing gets worse: every voice-control scenario and every test on the user's recordings passes (each command recognized exactly once; no command in ordinary speech, in a command phrase inside a sentence, in short Russian phrases or in "start stop"); the dictation still begins where the command's words end; no accuracy bar is loosened; the design states the extra load its checks put on the speech recognition service; `just full-check` passes.
- **Out of scope:**
  - The command set, what counts as a command, and the 0.3 s of silence that ends an utterance.
  - Dropping the confirmation by the speech recognition service for a start, unless the design shows that every accuracy and false-start scenario still passes without it.
  - The hotkey's recording start (push-to-talk latency).
  - A start after "stop" or "enter" in a chain during a dictation: it still waits for that dictation's paste, since commands are carried out in the order spoken.
  - The speech recognition model and its service settings; the time macOS takes to play a sound on a Bluetooth headset.
  - Bars for the other commands; they may get faster on the way ("cancel" through row 5).

## Release

PATCH: a bug fix. Target version **0.14.2** (from 0.14.1), or the next patch above `main` if another change releases first.
