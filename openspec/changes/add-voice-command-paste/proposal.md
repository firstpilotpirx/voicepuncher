## Why

Voice control has no way to paste what is already in the clipboard. The user often has text or other content copied and wants to insert it at the cursor by voice, without reaching for the keyboard. A spoken "paste" command fills that gap and works like the other voice control commands.

## What Changes

- New voice command "paste": when heard, voicepuncher pastes the current clipboard content at the current cursor position in the focused app.
- When the clipboard is empty, the focused app pastes nothing.
- "paste" joins the doubled short forms and the chains of the other standby commands, and gets a sound and an overlay sign of its own.

Current stage: PoC. Target stage: PoC.
Scope: one new voice command in voice control that pastes the existing clipboard content at the cursor.
Acceptance bar: saying "paste" while some text is in the clipboard inserts that text at the cursor in the focused app; the command is recognized like the other commands and does not misfire on ordinary dictation; component/E2E scenarios cover it.
Out of scope: changing what is in the clipboard, clipboard history, pasting non-text formatting rules, other new commands, a Russian form of the command, paste during a dictation.

## Capabilities

### Modified Capabilities

- `voice-control`: adds the "paste" command that pastes the clipboard content at the cursor.

## Impact

- Voice control command set in `app/` and the keyword list of the command spotting service in `command-spotting/`.
- Voice control specs, `.feature` scenarios and E2E scenarios for the new command.
- `docs/ubiquitous-language.md` (the voice command variants) and the README command list.

## Release

SemVer: MINOR — a new voice command is a new capability.
Target version: 0.22.0.
