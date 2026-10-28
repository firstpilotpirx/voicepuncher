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

## Decisions

- The command reads the clipboard as it is at that moment: dictation does not touch the clipboard until it pastes its own text on stop, so what is pasted is what the user copied, before or during the dictation.
- The clipboard text lands at the cursor at once; the dictated text lands where the cursor is when the dictation ends, as before. The order of the two is not part of this change.
- "paste" spoken during a dictation is removed from the dictated text.

## Release

SemVer: PATCH — bug fix.
Target version: 0.41.4.
