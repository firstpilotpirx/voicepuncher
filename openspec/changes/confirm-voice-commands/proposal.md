# Proposal

## Why

With voice control the user runs a whole set of commands hands-free, and for some of them it is crucial to know that they worked. Today "press enter" and "clear all text" change nothing in the overlay and share one command sound, so the user cannot tell whether a command ran, which one ran, or whether it was missed. The user asked first of all to see that a dictation was restarted, and to hear the confirmation as well as see it.

## What Changes

- After a voice command is carried out, the overlay briefly shows which command ran, so the user can see it without looking away from the app they dictate into.
- Every command plays its own sound, so commands can be told apart by ear; "press enter" and "clear all text" no longer share one sound.
- A restarted dictation is shown and sounds different from a fresh start, so the user knows the earlier speech was dropped.
- Decide which commands need the confirmation most. Commands with no visible effect of their own ("press enter", "clear all text", a restart) need it first; "start dictation", "stop dictation" and "cancel dictation" already change the overlay and play their sound, and keep doing so.
- Open question for explore: there is no "restart" command today. Settle whether to add one (discard the running dictation and start a new one in one phrase) or which existing command sequence the user meant.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: every carried-out command is confirmed on screen and by its own sound; possibly a new restart command.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: on-screen and sound confirmation of the voice commands, a restart of a dictation, and the choice of which commands get which confirmation.
- Acceptance bar: within a moment of each voice command the overlay names the command that ran and its own sound plays; a restart is visibly and audibly different from a start; a dictation that is not restarted looks and sounds as today.

## Out of scope

- Confirmations for hotkey actions in push-to-talk; they keep today's sounds and overlay.
- Spoken feedback and macOS system notifications.
- User-chosen sound files per command.

## Release

MINOR (new behavior), next minor version after the current one in `app/Cargo.toml` (0.9.0 → 0.10.0 at the time of writing).

## Impact

- The visual indicator gains a command confirmation signal; the overlay adapter draws it.
- The sound player gains a sound per command.
- The voice control step that carries out commands, and possibly the command spotting phrases for a restart command.
- `docs/ubiquitous-language.md`: `VisualIndicator` lists more than three signals.
- Manual run by the user (overlay, sounds).
