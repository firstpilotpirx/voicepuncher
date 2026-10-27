## Context

239 voice starts: hear 394 ms median, confirm 169, deliver 14, cue 274 (a second CoreAudio input opened before the overlay; the sound spawns `afplay`).

## Decisions

1. **Measure first.** The start log line gains `sound=` (until the sound plays) and `overlay=` (until the main thread drew the sign), and the service logs which path closed each segment (pause watch, half pause, voice detector, spotted check). The line is written from the overlay's draw callback (`show_recording` takes `on_drawn`). Later steps are judged on a replay of the user's voice samples.
2. **One microphone.** While voice control listens, the dictation recorder takes its audio from the kept microphone through `CpalAudioRecorder::with_source` (a fan-out of the keeper's audio); without voice control it opens the device as today. Sound and overlay go before `recorder.start()`.
3. **In-app sound.** `MacosSoundPlayer` preloads every cue as an `NSSound` at start instead of spawning `afplay`.
4. **Overlay windows kept.** `show_state` stops calling `do_rebuild`; the screen watcher rebuilds them on a screen change.
5. **Event-driven service loop.** The processing thread selects on audio and answers instead of a 20 ms poll; I/O read waits drop to 2 ms.
6. **Pause in noise.** The pause watch's quiet threshold follows the noise floor (quiet = below floor × 2 when the floor is above 3 % of the loudest frame), so a noisy room ends at the pause watch, not the voice detector's 0.85 s; tuned on voice samples mixed with room noise.
7. **Shorter pause.** `--min-silence-secs` 0.3 → 0.2; the half pause becomes 90 ms.
8. **Doubled forms at once.** When the spotter hears the same single-word command twice within 0.8 s, the service asks Whisper at once about the audio up to the second hit, without `CHECK_AFTER_SECS`, and reports a doubled form at once; existing paths stay as fallback.
9. **Smaller Whisper context.** The commands service gets `-ac` sized to its longest clip (50 per second); dictation keeps the full context.
10. **Confirmation switch.** `[voice_control] confirm_commands = true`; the app sends it in its `select` message (protocol version bump). Off: a spotter hit filling its segment alone, or a doubled hit, is reported at once as unconfirmed; its Whisper hearing still runs and goes out as a new `heard` event (agreed / other / none), logged by the app and recorded in the command history entry. Forms only Whisper places still wait.
11. **Rollback per step.** A step from 6–10 that adds a false start or loses accuracy in the voice-sample tests is reverted in its own commit with its spec text, and its measurement reported.

## Dependencies

`log-voice-command-history`: decision 10 writes Whisper's verdict into its command history entry and edits its `Session` and `EngineCfg`; applied after it merges.

## Risks / Trade-offs

- A 0.2 s pause can split "start … stop" said with a 0.25 s gap into a start and a stop.
- Without confirmation a misfire acts; it is the user's choice and recorded.
- `-ac` can raise hallucinations.

## Test levels

| Scenario | Level |
|---|---|
| Start start then silence starts at once | E2E |
| Stop stop in running speech stops at once | E2E |
| Start start inside a sentence starts at once | E2E |
| Start start without confirmation starts at once | E2E |
| A command inside a sentence still waits for confirmation | component |
| Confirmation is on by default | component |
| A command without confirmation records whether Whisper agreed | component |
| A false spotting without confirmation is recorded | component |
| Start start followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start said on its own gives its cue at once | E2E (`@cues`) |
| Restart restart followed by the new dictation at once gives its cue at once | E2E (`@cues`) |
| Restart said on its own gives its cue at once | E2E (`@cues`) |
| The user's recorded starts give their cue at once | E2E (`@voice_samples`) |
| The overlay shows with the start cue | component |
| A faster start keeps the dictation where the command's words end | E2E (existing) |
| A faster start adds no false start | E2E (existing) |
| Start please followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start please said on its own gives its cue at once | E2E (`@cues`) |
| A brief dip inside the command words starts nothing early | E2E (existing) |
| The start cue is timed once the other checks' builds have ended | E2E harness (existing) |
| A late start cue names what ran beside it | E2E harness (existing) |
| The start cue is timed against warm copies | E2E harness (existing) |
| A missing start cue names the confirmations cut short | E2E harness (existing) |
| Stop after a short breath | E2E (existing) |
| Restart, a short pause, then speech that starts like another word | component + E2E (existing) |
| A command alone, a short pause, then speech | E2E |
| The user's recorded command alone, a short pause, then speech | E2E (`@voice_samples`) |
| A command in a noisy room ends at its pause | E2E |
| A voice start logs its timing | component |
| A start without confirmation logs that it was not confirmed | component |
| The timing stays right after microphone audio was lost | component (existing) |
| A hotkey start logs no voice start timing | component (existing) |
| The recording sign is drawn at once | adapter integration (`needs_desktop`) |
| A screen added later gets the overlay | adapter integration (`needs_desktop`) |
| The switch is on by default | component (settings) |
| Switching it off applies to the next command | component (settings) |

Pause watch threshold, doubled-hit detection and the `heard` event also get unit tests in `command-spotting/`.
