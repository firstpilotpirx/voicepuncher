## Why

The user's clipboard is lost after a dictation. Steps the user reported: while a recording runs, the user copies something to the clipboard, for example a screenshot (an image, not text); then the user ends the recording by voice ("закончи запись"); the app ends the recording, recognizes and pastes the text; afterwards the clipboard is empty — the screenshot the user copied is gone. The user expects the clipboard to hold what they put there, as it does when the app is not used.

## What Changes

- After the app pastes a dictation, the clipboard again holds what the user last put there, including non-text content such as an image, also when the user copied it while the recording was running.

Current stage: PoC. Target stage: PoC.
Scope: how the paste step saves and restores the user's clipboard around its own paste.
Acceptance bar: the reported steps (copy a screenshot during a recording, end it by voice, paste happens) leave the screenshot in the clipboard, on the user's manual run; an automated test covers restoring non-text clipboard content and content copied during the recording.
Out of scope: the overlay, voice command recognition, clipboard history.

## Capabilities

### Modified Capabilities

- `text-insertion`: the paste restores the user's clipboard, every content type, including content copied while the recording ran.

## Impact

- The paste adapter (clipboard save and restore around the paste) and its tests.
- Manual run by the user (paste change).

## Open points

- Whether the loss comes from the clipboard being saved at the start of the recording (so a later copy is overwritten), from only text types being saved and restored (so an image is dropped), or from the voice end path ("закончи запись") skipping the restore. Proposed: find out first; restore every type of the clipboard as it stands right before the paste.
- The capability name: proposed `text-insertion`; use the existing capability that owns the paste.
- Whether it happens only with the voice end command or also with the hotkey: the user reported the voice command.

## Release

SemVer: PATCH — a bug fix.
Target version: 0.24.1 (or the next PATCH after fix-overlay-glass-glitches, whichever lands first).
