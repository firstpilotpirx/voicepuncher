# Design

## Context

Voice control carries out commands in `Pipeline` (`pipeline/voice_control.rs`). "press enter" and "clear all text" call `SoundPlayer::play_command` after the key press; "restart dictation" is `cancel_recording` (cancel sound, overlay hidden) followed by `start_recording_from` (start sound, recording shown). The macOS overlay shows text signs ("◉ REC", "⌨ FOCUS") and cancels its delayed work with a generation counter. The `[sounds]` config table denies unknown fields and has one `command_sound` (Morse).

## Decisions

**1. A new `VisualIndicator::show_command_sign(VoiceCommand)` signal.** The core calls it after the key press of "press enter" / "clear all text" and after the restart's new recording started. The adapter maps the command to its sign ("⏎ ENTER", "⌫ CLEAR", "↻ RESTART"; other commands have none) and, about 1 s later, returns to the state it held when the sign came: hidden, or the recording. Any later `show_*` or `hide` bumps the generation and cancels that return, so in a chain the next sign or the recording replaces the sign at once. Rejected: a timer in the core — the overlay already owns delayed display work, and the core would need a clock-driven input for a purely visual matter.

**2. A sound per command on `SoundPlayer`: `play_enter`, `play_clear`, `play_restart`** replace `play_command`. Rejected: `play_command(VoiceCommand)` — it would accept commands that have no sound of their own.

**3. Restart without the cancel and start cues.** `cancel_recording` and `start_recording_from` keep their cues for every other path; their recorder and live-session work moves into helpers (`discard_recording`, and a start that takes the cue to play), so restart discards silently, starts with `play_restart`, shows the recording, then the restart sign. Rejected: suppressing sounds with a pipeline flag — hidden state that every later edit of the two paths must remember.

**4. Config: `enter_sound` (alias `command_sound`, default Morse), `clear_sound` (default Purr), `restart_sound` (default Hero).** The alias keeps an existing config file with `command_sound` loading despite `deny_unknown_fields`. Purr and Hero are system sounds no other cue uses by default. The setup wizard does not ask for them (proposal).

**5. Glossary.** `VisualIndicator` gains the command sign as its sixth signal; `SoundPlayer` lists the per-command sounds; a new term *Command sign*.

## Dependencies

None declared yet; stage "dependencies" compares with the running overlay changes.

## Risks / Trade-offs

- [The 1 s return races a later signal] → the generation check runs on the main queue, where every overlay update runs, so a stale return is dropped.
- [Enter during a dictation pastes asynchronously after focus waits] → the sign is shown after `stop_recording` returns, when the paste flow has finished, as the key press is today.
- [Other changes in flight also add overlay signals] → see § Dependencies.

## Test levels

| Spec scenario | Level |
|---|---|
| Saying press enter presses Return | component `.feature` (sound and sign called; the 1 s return: adapter unit test of the return-state choice, manual run) |
| Saying clear all text selects all and deletes | component `.feature` |
| Saying enter during a dictation pastes and sends | component `.feature` |
| Saying restart drops the dictation and starts a new one | component `.feature` |
| Only the speech after a restart is pasted | component `.feature` (unchanged) |
| Restart restart in the middle of speech | E2E `.feature` (unchanged) |
| Enter and clear sound different by default | unit (config defaults) |
| A sound the user picked for a command plays | unit (config parse, alias) + manual run |
| Enter start shows the recording right after the enter sign | component `.feature` |
| Stop clear enter ends on the enter sign | component `.feature` |
| A hotkey dictation shows no command sign | component `.feature` |
