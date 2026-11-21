## Why

Dictation in voicepuncher garbles English terms and fast speech: the same text dictated through voicepuncher came out with about 15 meaning errors ("заассайни" → "за сайне", "ишью" → "шью", "API-шки" → "опишки", "моя распознаёт" → "моя спазм"), while Claude Code's own voice input recognized it almost perfectly. Paid cloud recognition is rejected (even $3/month is too much for ~17 h of dictation a month). Claude Code's voice input is free for the user, so inside Claude Code voicepuncher should drive it instead of its own recognizer (the user's request when filing this change, 2026-11-18). voicepuncher itself still sends nothing off the machine: it only presses Claude's key, as the user would; in Claude, Claude's own dictation records by the user's choice.

## What Changes

- When the frontmost app is the Claude desktop app (bundle id `com.anthropic.claudefordesktop`, where the user runs Claude Code in its Code tab), a dictation start — "start dictation" or the hotkey — presses Claude's own dictation shortcut (Cmd+D by default, a toggle) instead of recording; "stop dictation" or the hotkey presses it again, so Claude records, recognizes and inserts the text into its composer itself.
- "cancel dictation" presses Escape (Claude drops its dictation); "restart dictation" presses Escape, then the shortcut; "press enter" during it presses the shortcut, waits for Claude's text, then presses Return.
- The start, stop and cancel cues and the recording overlay stay as with the app's own dictation; voicepuncher does not record, transcribe, paste or archive such a dictation. In every other app dictation works as today.
- When the shortcut cannot be posted, voicepuncher falls back to its own dictation and logs why; the decision is taken at each start, by the frontmost app.
- A new section `[claude_dictation]` with `enabled` (on by default) and `shortcut` (`cmd+<letter or digit>`, default `cmd+d`), both on the Dictation tab of the Settings window (the shortcut under the advanced disclosure).

Current stage: PoC. Target stage: PoC.
Scope: the facade for the Claude desktop app's own dictation, driven by voice commands and the hotkey; the choice between it and the app's own dictation by the frontmost app at each start; a setting to turn it off and one for its shortcut. Settled at explore: the terminal Claude Code CLI is not covered — its voice mode must be turned on by `/voice`, its push-to-talk key is Space with an empty input, and its hold mode needs key repeats that a synthetic held key does not produce.
Acceptance bar: in Claude Code, voice start → speak → "stop" puts Claude Code's own transcript into the chat; in another app dictation is unchanged; the manual run shows both.
Out of scope: changing the local recognizer; voicepuncher's own paid or cloud recognition; other apps' voice buttons.
Interface: desktop GUI — two new rows on the Dictation tab of the Settings window; the recording overlay and cues look as today.

## Capabilities

### Modified Capabilities

- `voice-control`: in Claude, a voice start and the commands that end a dictation drive Claude's own dictation; the start, stop, enter and hotkey requirements gain that exception.
- `push-to-talk`: in Claude, the hotkey drives Claude's own dictation; the Hold-mode requirement gains that exception.

## Impact

- Hotkey/voice start handling, the focused-app check, a key-press adapter, configuration `[claude_dictation]`, the Settings window, README, `docs/architecture/overview.md` and `AGENTS.md` (the offline wording), `docs/project-map.md`.

## Decisions (settled at explore)

- Claude's dictation key: the Claude desktop app's dictation shortcut, Cmd+D by default (its settings name it `dictationShortcut`, a toggle: press to start, press to stop); kept in `[claude_dictation] shortcut` in case the user or an update changes it.
- How Claude is recognized: the frontmost app's bundle id is `com.anthropic.claudefordesktop`; no window title is needed, as the CLI in a terminal is out of scope.
- A press that Claude ignores cannot be seen from outside; only a press that cannot be posted falls back (no Accessibility stops the whole pipeline at the permissions gate before any dictation) to the app's own dictation.
- The cues and the overlay stay on in this mode.
- Voice-start timing stays: a voice start in Claude presses the shortcut before its cue, still logs `voice start: total=…ms` and keeps the 0.7 s bound.
- Known limit: Claude's dictation hears the user too, so a spoken "stop dictation" ends up at the end of Claude's text; the hotkey has no such words.

## Release

SemVer: MINOR — new behavior in voice control.
Target version: 0.50.0.
