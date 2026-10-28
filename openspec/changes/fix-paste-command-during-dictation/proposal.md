## Why

The voice command "paste" works only when dictation is off. While dictation is running, saying "paste" does nothing. The user needs it during dictation too: while dictating, they may want to insert what is on the clipboard — something copied before dictation started, or copied while it runs — without stopping the dictation.

## What Changes

- Saying "paste" while dictation is running pastes the current clipboard contents at the cursor, exactly as it does with dictation off.
- What is pasted is the user's clipboard at the moment of the command (copied before dictation started or during it), never the dictated text.
- Dictation is not interrupted or stopped by the command: it goes on recording and transcribing after the paste.

Current stage: PoC. Target stage: PoC.
Scope: the "paste" voice command while dictation is running.
Acceptance bar: with dictation running, copy a text, say "paste" — that text appears at the cursor, and dictation keeps going and later delivers its own text; a test proves the command is carried out during dictation.
Out of scope: other voice commands during dictation; the behavior of "paste" with dictation off.

## Capabilities

### Modified Capabilities

- `voice-control`: the "paste" command is carried out while dictation is running, without interrupting it.

## Impact

- The voice command handling of the app and its interplay with a running dictation (pasting the dictated text uses the clipboard too).

## Open points

- Dictation's own paste of its text also goes through the clipboard: the clipboard contents the user had must not be replaced by dictated text at the moment of the command, and the user's clipboard must survive dictation's paste. Proposed: the "paste" command reads the user's clipboard as it is, and dictation's paste keeps restoring the user's clipboard afterwards.
- Where the pasted text lands relative to dictated text not yet pasted (dictation pastes on stop or per chunk). Proposed: at the cursor at the moment of the command; dictated text comes after it.
- Whether "paste" spoken during dictation should be removed from the dictated text. Proposed: yes, the command word does not appear in the dictation.

## Release

SemVer: PATCH — bug fix.
Target version: 0.41.4.
