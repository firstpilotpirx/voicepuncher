## Why

Dictation in voicepuncher garbles English terms and fast speech: the same text dictated through voicepuncher came out with about 15 meaning errors ("заассайни" → "за сайне", "ишью" → "шью", "API-шки" → "опишки", "моя распознаёт" → "моя спазм"), while Claude Code's own voice input recognized it almost perfectly. Paid cloud recognition is rejected (even $3/month is too much for ~17 h of dictation a month). Claude Code's voice input is free for the user, so inside Claude Code voicepuncher should drive it instead of its own recognizer.

## What Changes

- When the focused app is Claude Code, a voice start (or the hotkey) makes voicepuncher press and hold Claude Code's own voice-record key; "stop" releases it, so Claude Code records, recognizes and inserts the text into its chat itself.
- voicepuncher does not record, transcribe or paste in that case; in every other app dictation works as today.
- When the facade cannot work (key not accepted, Claude Code not in focus), voicepuncher falls back to its own dictation and logs why.

Current stage: PoC. Target stage: PoC.
Scope: the facade for Claude Code's voice recording, the choice between facade and own dictation by the focused app, a setting to turn the facade off.
Acceptance bar: in Claude Code, voice start → speak → "stop" puts Claude Code's own transcript into the chat; in another app dictation is unchanged; the manual run shows both.
Out of scope: changing the local recognizer; any paid or cloud recognition; other apps' voice buttons.
Interface: desktop GUI (Claude Code chat, the recording overlay).

## Capabilities

### Modified Capabilities

- `voice-control`: voice start and "stop" drive Claude Code's voice recording when Claude Code is focused.

## Impact

- Hotkey/voice start handling, the focused-app check, a key-press adapter, configuration (`[claude_code_facade]` or similar), README release checklist.

## Open points

- Which key Claude Code uses for voice recording (CLI: hold Space? desktop app: the mic button or a shortcut) and whether it is push-to-talk (hold) or toggle — proposed: find out at explore and support the one the user uses (desktop app Code tab and/or terminal CLI).
- How to tell Claude Code is focused (bundle id of the Claude desktop app, a terminal window running `claude`) — proposed: bundle id plus window title.
- The key is undocumented and may change with a Claude Code update — proposed: key in config, fallback to own dictation with a log line.
- Whether the start/stop cue sounds and the overlay stay on in facade mode — proposed: keep them.

## Release

SemVer: MINOR — new behavior in voice control.
Target version: 0.50.0.
