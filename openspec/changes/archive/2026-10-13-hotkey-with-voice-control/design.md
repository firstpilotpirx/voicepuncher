# Design

## Context

Since 0.7.0 `Pipeline::handle_with_voice_control` maps a hotkey press to `toggle_listening` and ignores the release, so with voice control on the hotkey never records. Both start paths already share `Pipeline::start_recording`, `stop_recording` and `cancel_recording`, which play the push-to-talk sounds and drive the overlay. Yet in the user's manual run of a voice dictation neither the overlay nor the start sound appeared; the cause is not visible in the code and is found first.

## Decisions

### 1. Hotkey signals go through the push-to-talk step
With voice control on, `PipelineInput::Hotkey` goes to the same `step` as without it: press, release (hold mode) and Escape do what they do in push-to-talk. `toggle_listening` and the listening-off path of the hotkey go away; `turn_listening_off` stays for a lost service.
- Alternative: a second hotkey for listening — not asked for; quitting the app or running without voice control turns listening off.

### 2. The phrase set follows the dictation, whatever started it
`start_recording`, `stop_recording` and `cancel_recording` switch the service to the dictation phrases or back to the standby phrases when voice control is on, instead of the voice command handlers doing it. The switch to the dictation phrases happens before the recorder starts, as in 0.8.0, and is undone when the recorder fails to start. So "stop dictation" ends a hotkey dictation and the hotkey ends a voice dictation.

### 3. The overlay and sounds of a voice dictation are pinned by tests
The adapters log each overlay and sound call at debug level. In the user's run of 0.8.0 with the logs on, a voice dictation played the start sound and showed the overlay (`overlay: recording shown on 1 screen(s)`), so the earlier missing overlay was not reproduced; the earlier run was the 0.7.0 app from the main checkout. Component scenarios now record the overlay in the shared event log and assert it for a voice dictation: recording shown on start, processing on stop, hidden after the paste; and the start and stop sounds.

### 4. Short forms are matcher aliases only
"enter", "clear", "start" and "stop" are aliases in `PhraseMatcher`, so only the whisper-confirmed path reports them; the keyword spotter gets no one-word keywords, which it would spot inside ordinary speech. The pause rule and the per-state phrase sets still apply: "stop" counts only during a dictation, "start", "enter" and "clear" only without one. "cancel" alone stays nothing, since it is common in speech and discarding a dictation by mistake loses it.

## Test plan

| Spec scenario | Level |
|---|---|
| The hotkey records when voice control is off | component `.feature` (unchanged) |
| The hotkey records when voice control is on | component `.feature`; pipeline unit |
| The hotkey ends its dictation when voice control is on | component `.feature` |
| Escape cancels a dictation when voice control is on | component `.feature` |
| A hotkey press ends a voice dictation and keeps listening | component `.feature` |
| Saying stop dictation ends a hotkey dictation | component `.feature`; pipeline unit (phrase set switches on hotkey start) |
| Saying cancel dictation discards a hotkey dictation | component `.feature` |
| Listening is on at start | component `.feature` (unchanged) |
| Listening stays on after a hotkey dictation | component `.feature` |
| Saying start dictation starts a dictation | component `.feature` (adds the overlay) |
| Saying stop dictation plays the stop sound and shows processing | component `.feature` |
| A short form is recognized as its command | unit (matcher); E2E `.feature` (`say`, real service) |
| A short Russian phrase is no command | E2E `.feature` ("стоп" removed from the phrases) |
| Every recorded command of the user is recognized | adapter integration (`voice_samples_it`, rerun with the short forms) |

## Risks / Trade-offs

- [Listening can no longer be paused by hotkey] → the user asked for push-to-talk as before; quitting or running without `--voice-control` stops listening.
- [Hold mode with voice control: the release ends a voice-started dictation] → same as push-to-talk: in hold mode a press during a dictation is ignored and the release ends it, whatever started the dictation.
