# Proposal

## Why

With voice control the user runs a whole set of commands hands-free, and for some of them it is crucial to know that they worked. Today "press enter" and "clear all text" change nothing in the overlay and share one command sound, so the user cannot tell whether a command ran, which one ran, or whether it was missed. The user asked first of all to see that a dictation was restarted, and to hear the confirmation as well as see it.

## What Changes

- After "press enter", "clear all text" or "restart dictation" is carried out, the overlay shows a sign naming that command for about a second ("⏎ ENTER", "⌫ CLEAR", "↻ RESTART"), so the user can see it without looking away from the app they dictate into. The sign gives way at once to what comes next in a chain.
- Each of the three commands plays its own sound, so commands can be told apart by ear; "press enter" and "clear all text" no longer share one sound.
- A restarted dictation plays one restart sound instead of the cancel sound followed by the start sound, and shows its sign before the recording, so the user knows the earlier speech was dropped.
- The three command sounds can be changed in the config file, like the other sounds; the setup wizard does not ask for them.
- "start dictation", "stop dictation" and "cancel dictation" already change the overlay and play their sound, and keep doing so unchanged. ("restart dictation" exists since 0.10.0, so the open question about it is settled.)

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: "press enter", "clear all text" and "restart dictation" are confirmed on screen and by a sound of their own.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: on-screen and sound confirmation of "press enter", "clear all text" and "restart dictation".
- Acceptance bar: within a moment of "press enter", "clear all text" or "restart dictation" the overlay names the command that ran and its own sound plays; a restart is visibly and audibly different from a start; a dictation that is not restarted looks and sounds as today.

## Out of scope

- Confirmations for hotkey actions in push-to-talk; they keep today's sounds and overlay.
- Spoken feedback and macOS system notifications.
- User-chosen sound files per command.

## Release

MINOR (new behavior): 0.16.0, the next minor version after 0.15.3.

## Impact

- The visual indicator gains a command confirmation signal; the overlay adapter draws it.
- The sound player gains a sound per command.
- The voice control step that carries out commands; the config gains three command sounds.
- `docs/ubiquitous-language.md`: `VisualIndicator` lists more than three signals.
- Manual run by the user (overlay, sounds).
