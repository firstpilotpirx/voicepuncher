## Context

239 voice starts: hear 394 ms median, confirm 169, deliver 14, cue 274 (a second CoreAudio input opened before the overlay; the sound spawns `afplay`).

Before numbers (the user's log, 239 voice starts, median / one in ten): total 837 / 1226 ms, hear 394 / 642, confirm 169 / 271, deliver 14 / 37, cue 274 / 323.

## Decisions

1. **Measure first.** The start log line's `cue=` is the time until the sound played, and it gains `overlay=` (until the main thread drew the sign), and the service logs which path closed each segment (pause watch or voice detector; a half pause and a spotted check close none, they only ask confirmations). The line is written from the overlay's draw callback (`show_recording` takes `on_drawn`). Later steps are judged on a replay of the user's voice samples.
2. **One microphone.** While voice control listens, the dictation recorder takes its audio from the kept microphone through `CpalAudioRecorder::with_source` (a fan-out of the keeper's audio); without voice control it opens the device as today. Sound and overlay go before `recorder.start()`.
3. **In-app sound.** `MacosSoundPlayer` preloads every cue as an `NSSound` at start instead of spawning `afplay`.
4. **Overlay windows kept.** `show_state` stops calling `do_rebuild`; the screen watcher rebuilds them on a screen change.
5. **Event-driven service loop.** Each confirmation thread wakes the processing thread right after it sends an answer (an `Answered` input), so an answer is applied at once instead of at the next audio message or 20 ms poll, which stays as a backstop; I/O read waits drop to 2 ms. The app's microphone keeper hands its audio on every 10 ms instead of 20.
6. **Pause in noise.** The pause watch's quiet threshold already follows the noise floor (quiet = at most 3 × the quietest frame of the last 3 s, capped at 30 % of the loudest frame), so a noisy room ends at the pause watch, not the voice detector's 0.85 s; a unit test pins it with noise at a fifth of the voice, and the E2E scenario times a start in steady noise. No code change.
7. **Shorter pause — reverted.** `--min-silence-secs` was to go 0.3 → 0.2. Measured (decision 11): with 0.2 s the command spotting service lost 4 of 80 recorded forms (a doubled "cancel cancel", "enter" during a dictation, a chain of three, a false stop) and 1 of 37 recorded commands; with 0.3 s on the same services both tests passed. The pause stays 0.3 s. The 0.5 s cue bound leaned on the shorter pause: `just e2e-cues` measured 0.64 s for the user's recorded "start" alone and 0.67 s for "restart" alone, so every form but a doubled one keeps the 0.7 s bound, and the requirement keeps its heading.
8. **Doubled forms at once.** When the spotter hears the same single-word command twice within 0.8 s, the service asks Whisper at once about the audio up to the second hit, without `CHECK_AFTER_SECS`, and reports a doubled form at once; existing paths stay as fallback. The keyword spotter has single-word triggers for "stop" and "cancel" too, which count only doubled: it often spots "stop stop" once, near the end of the second word, so a short "stop" or "cancel" asks the doubled check 0.15 s after it, and two hits within 0.8 s ask it at once. Before this `just e2e-cues` measured "stop stop" in running speech at 1.7 s, found only by the window check; a doubled trigger also asks its check when confirmation is on.
9. **Smaller Whisper context — reverted.** The commands service was to get `-ac` sized to its longest clip (50 per second). Measured (decision 11): with `-ac 1100` the services `start-model-servers` runs lost four of the six real-service confirmation tests (`placed_commands::needs_command_stt`) and 14 of 80 recorded forms, deterministically; with `-ac 0` on the same services all six passed. Both services keep the full context.
10. **Confirmation switch.** `[voice_control] confirm_commands = true`; the app sends it in its `select` message (protocol version bump). Off: a spotter hit filling its segment alone, or a doubled hit, is reported at once as unconfirmed; its Whisper hearing still runs and goes out as a new `heard` event (agreed / other / none), logged by the app and recorded in the command history entry. Forms only Whisper places still wait.
11. **Rollback per step.** A step from 6–10 that adds a false start or loses accuracy in the voice-sample tests is reverted in its own commit with its spec text, and its measurement reported.

## After numbers

`just e2e-cues` on 82e7665b, idle machine, from the end of the command's words to the start cue the command spotting service gives (the app's own cue, before 274 ms median, is measured in the manual run):

| Form | Before bound | Measured |
|---|---|---|
| "start start" alone, with and without confirmation / followed by speech | 0.7 s | 227–244 / 196 ms |
| "fine start start" followed by speech | — | 303 ms |
| "stop stop" inside speech (was 1 706 ms) | — | 285 ms |
| "restart restart" followed by speech | 0.7 s | 453 ms |
| "start" alone / followed by speech | 0.7 s | 308 / 492 ms |
| "start please" alone / followed by speech | 0.7 s | 129 / 411 ms |
| "restart" alone after 20 s of dictation | 0.7 s | 527 ms |
| "start" in room noise | — | 176 ms |
| the user's 12 recorded starts | 9 of 10 within 0.7 s | 11 of 12 within their bound; doubled 176–454 ms, others 221–746 ms (a recorded "start please" followed by speech over) |

Steps 2–5 and 8 stay; steps 7 (pause 0.2 s) and 9 (`-ac 1100`) are reverted (above).

## Dependencies

`log-voice-command-history`: decision 10 writes Whisper's verdict into its command history entry and edits its `Session` and `EngineCfg`; applied after it merges.

## Risks / Trade-offs

- Without confirmation a misfire acts; it is the user's choice and recorded.

## Test levels

| Scenario | Level |
|---|---|
| Start start then silence starts at once | E2E |
| Stop stop in running speech stops at once | E2E |
| Start start inside a sentence starts at once | E2E |
| Start start without confirmation starts at once | E2E |
| A command inside a sentence still waits for confirmation | E2E (service, `@smoke`) |
| Confirmation is on by default | E2E (service, `@smoke`) |
| A command without confirmation records whether Whisper agreed | E2E (service, `@smoke`) |
| A false spotting without confirmation is recorded | E2E (service, `@smoke`) |
| Start start followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start said on its own gives its cue at once | E2E (`@cues`) |
| Restart restart followed by the new dictation at once gives its cue at once | E2E (`@cues`) |
| Restart said on its own gives its cue at once | E2E (`@cues`) |
| The user's recorded starts give their cue at once | E2E (`@voice_samples`) |
| The overlay shows with the start cue | component |
| A faster start keeps the dictation where the command's words end | E2E (existing) |
| A faster start adds no false start | E2E (existing); "start stop" at service integration (existing) |
| Start please followed by the dictation at once gives its cue at once | E2E (`@cues`) |
| Start please said on its own gives its cue at once | E2E (`@cues`) |
| A brief dip inside the command words starts nothing early | service integration (existing) |
| The start cue is timed once the other checks' builds have ended | E2E harness (existing) |
| A late start cue names what ran beside it | E2E harness (existing) |
| The start cue is timed against warm copies | E2E harness (existing) |
| A missing start cue names the confirmations cut short | E2E harness (existing) |
| Stop after a short breath | E2E (existing) |
| Restart, a short pause, then speech that starts like another word | E2E (existing) |
| A command alone, a short pause, then speech | E2E |
| The user's recorded command alone, a short pause, then speech | E2E (`@voice_samples`) |
| A command in a noisy room ends at its pause | E2E |
| A voice start logs its timing | component |
| A start without confirmation logs that it was not confirmed | component |
| The timing stays right after microphone audio was lost | component (existing) |
| A hotkey start logs no voice start timing | component (existing) |
| The recording sign is drawn at once | adapter integration (`needs_desktop`) |
| A screen added later gets the overlay | component + adapter integration (`needs_desktop`) |
| The switch is on by default | component (settings) |
| Switching it off applies to the next command | component (settings) |

Pause watch threshold, doubled-hit detection and the `heard` event also get unit tests in `command-spotting/`.
