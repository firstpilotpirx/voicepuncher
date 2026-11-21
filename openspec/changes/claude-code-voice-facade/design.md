## Context

Claude Code runs in the Code tab of the Claude desktop app (bundle id `com.anthropic.claudefordesktop`). The app has its own dictation, started and stopped by one shortcut: its settings key `dictationShortcut` defaults to Cmd+D on macOS (a Ctrl+D elsewhere), and a custom one must hold Command or Control. While its dictation is live, Escape belongs to the dictation (`dictationOwnsEscape`) and drops it. Its recognition is far better on the user's mixed Russian and English speech than the local whisper (proposal § Why).

The terminal Claude Code CLI has a voice mode too (`/voice hold|tap`, action `voice:pushToTalk`, Space by default, tap mode only with an empty input), but it must be turned on first, and its hold mode reads key repeats, which a synthetic held key does not send: it stays out of this change.

The pipeline (`app/src/core/application/pipeline/`) starts every dictation in `start_recording_from`, ends it in `stop_recording` and drops it in `abandon` (cancel, restart, shutdown); voice commands and the hotkey both reach these three. `KeyPresser` and `keystroke::press` already post key presses as a keyboard does.

## Goals / Non-Goals

**Goals:**
- In Claude, every way a dictation starts, stops, cancels or restarts drives Claude's own dictation, with voicepuncher's cues and overlay.
- Everywhere else, nothing changes.

**Non-Goals:**
- The terminal CLI's voice mode; other apps' voice buttons.
- Removing the spoken "stop dictation" from Claude's text (Claude inserts it itself).
- Telling whether Claude acted on the press.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Which key starts and stops Claude's dictation | its shortcut, configured as `[claude_dictation] shortcut`, default `cmd+d`; pressed once to start, once to stop | the desktop app's own default; a toggle needs no held key | hard-coded key (the user or an update may change it); holding a key (a toggle has nothing to hold) |
| Which shortcuts are accepted | `cmd+` and one letter or digit, parsed into a `KeyChord` (domain) | `keystroke` holds only Command; Claude refuses a shortcut without Command or Control | Control or Option chords (left Control and Option can be the hotkey, which hears the presses) |
| How Claude is recognized | the frontmost app's bundle id, read through `NSWorkspace.frontmostApplication` at each start | one call, no window title needed without the CLI | the Accessibility focused element (says nothing about the app); window titles |
| Where the choice lives | a port `ClaudeDictation` in `core/application` (`is_frontmost`, `toggle`, `cancel`); the pipeline asks it at each start and keeps the answer until the dictation ends | the core stays free of AppKit; one place decides | a check in the hotkey adapter (misses voice starts) |
| How the pipeline keeps a Claude dictation | a new module `pipeline/claude_dictation.rs`; the `Pipeline` gets one field holding the port and whether a Claude dictation runs; `is_recording` counts it | the existing `Dictation` holds a recorder session, live chunks and an archive trace, none of which apply | a variant inside `Dictation` (every recorder path would need a guard) |
| Order at a start | press the shortcut first; only when it was posted, play the start cue, show the overlay and point the command spotting at the dictation phrases | a failed press falls back to the app's own dictation with no doubled cue | cue first (a fallback would play it twice) |
| Stop | press the shortcut, play the stop sound, hide the overlay at once (no processing overlay: nothing is transcribed here) | voicepuncher has nothing left to wait for | showing processing until a paste that never comes |
| Cancel and restart | Escape drops Claude's dictation; restart is Escape, then the shortcut, with the restart cue | Escape belongs to a live Claude dictation | pressing the shortcut (it would keep the text) |
| "press enter" during it | shortcut, then Return after 1.5 s (`CLAUDE_TEXT_WAIT`) | Claude needs time to finish recognition and insert the text | Return at once (it would send before the text arrives) |
| What "stop dictation" truncates | nothing: `close_dictation` cuts the recorder only for the app's own dictation | no recorder runs | |
| Fallback | a press that fails (no Accessibility) logs `claude dictation: not used: <why>` and the app's own dictation starts; `enabled = false` builds no port | dictation is never lost | a failure notice only |
| Archive and history | a Claude dictation is not archived | voicepuncher holds no audio or text of it | an empty archive entry |
| Settings window | Dictation tab: switch `Use Claude's own dictation in Claude` (`claude_dictation.enabled`), and under the advanced disclosure `Claude dictation shortcut` (`claude_dictation.shortcut`) as a text field, refused when `KeyChord::parse` fails | the Settings window shows every option | a separate tab |
| Default | on | the user asked for it; it changes only dictations in Claude | off |
| Offline rule | voicepuncher only presses a key; Claude's dictation is Claude's own, as when the user presses Cmd+D | the app itself still opens no connection beyond loopback | |

### Names (ubiquitous language)

- **Claude dictation** — the Claude desktop app's own dictation, which voicepuncher starts, stops and drops by Claude's shortcut and Escape when Claude is the frontmost app at a dictation's start. Port `ClaudeDictation`, adapter `adapters/secondary/claudedictation/macos/`.
- **KeyChord** — a Command shortcut of one letter or digit, as the configuration writes it (`cmd+d`).

## Test levels

| Scenario | Level |
|---|---|
| Saying start in Claude starts Claude's dictation | component (`voice_control.feature`, stand-in `ClaudeDictation`) |
| Saying stop in Claude stops Claude's dictation | component (`voice_control.feature`) |
| Saying cancel in Claude drops Claude's dictation | component (`voice_control.feature`) |
| Saying restart in Claude starts Claude's dictation over | component (`voice_control.feature`) |
| Saying enter in Claude stops and sends | component (`voice_control.feature`) |
| Another app keeps the app's own dictation | component (`voice_control.feature`) |
| The Claude dictation turned off records as before | component (`voice_control.feature`) |
| A shortcut that cannot be pressed falls back | component (`voice_control.feature`) |
| A shortcut without Command is refused | unit (`app/src/config/tests.rs`, `KeyChord` tests) |
| Toggle mode in Claude | component (`push_to_talk.feature`) |
| Hold mode in Claude | component (`push_to_talk.feature`) |

The real Claude app and the real key presses are checked by the manual run (tasks 4.4): no automated test may press Cmd+D in the user's apps.

## Interface mock

[Settings window, Dictation tab, light and dark](mock/settings-dictation.html) — the two new rows; the overlay and cues look as today.

## Risks / Trade-offs

- A spoken "stop dictation" (and "press enter") reaches Claude's text, since Claude hears the user too → documented in the README; the hotkey has no such words.
- A press Claude ignores (the shortcut changed in Claude, no composer in view) looks like a dictation to voicepuncher → the overlay still shows recording, and the user sees no text; the shortcut is configurable, and the log names every press.
- `NSWorkspace.frontmostApplication` read off the main thread → it is updated by the main run loop, which the menu bar app runs; the adapter test reads it on a test thread.
- The Claude desktop app may change its shortcut or Escape handling in an update → only `shortcut` needs changing; Escape is the documented way to drop its dictation.
