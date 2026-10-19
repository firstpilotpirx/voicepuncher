## Why

Voice control already has a set of spoken commands, but there is no way to paste what is already in the clipboard. The user often has text or other content copied and wants to insert it at the cursor by voice, without reaching for the keyboard. A spoken "paste" command fills that gap and works like the other voice control commands.

## What Changes

- New voice command "paste": when heard, voicepuncher pastes the current clipboard content at the current cursor position in the focused app, the same way the other voice control commands act.
- When the clipboard is empty, the command does nothing to the focused app.

Current stage: PoC. Target stage: PoC.
Scope: one new voice command in voice control that pastes the existing clipboard content at the cursor.
Acceptance bar: saying "paste" while some text is in the clipboard inserts that text at the cursor in the focused app; the command is recognized like the other commands and does not misfire on ordinary dictation; component/E2E scenarios cover it.
Out of scope: changing what is in the clipboard, clipboard history, pasting non-text formatting rules, other new commands.

## Capabilities

### Modified Capabilities

- `voice-control`: adds the "paste" command that pastes the clipboard content at the cursor.

## Impact

- Voice control command set in `app/` and the keyword list of the command spotting service in `command-spotting/`.
- Voice control specs, `.feature` scenarios and E2E scenarios for the new command.
- `docs/ubiquitous-language.md` if the command needs a new term; README command list.

## Open points

- Spoken word(s): only "paste", or also a Russian form (e.g. «вставь»)? Proposed: the same language pattern the existing commands use.
- Clipboard content other than text (images, files): paste whatever the system clipboard holds via Cmd+V, or text only? Proposed: send the system paste (Cmd+V) and let the focused app handle the content.
- Does the paste command keep the dictation clipboard-restore behavior intact (the app pastes dictated text through the clipboard)? Proposed: the command pastes the user's clipboard as it is, without touching it.
- Where the command is allowed (the command placements used by existing commands, e.g. only as a standalone utterance vs. inside dictation). Proposed: same placements as the other action commands.
- Target version: MINOR bump to 0.22.0, adjusted if `main` moves first.

## Release

SemVer: MINOR — a new voice command is a new capability.
Target version: 0.22.0.
