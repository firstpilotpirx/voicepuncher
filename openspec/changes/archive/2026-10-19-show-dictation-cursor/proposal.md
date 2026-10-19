# Proposal

## Why

While dictating the user looks at the place the text goes to, not at the overlay. A mouse pointer that changes to a fitting sign while the user speaks shows at a glance, where the user already looks, that a recording runs.

## What Changes

- While a recording runs, the mouse pointer shows a dictation sign (for example a microphone) wherever it is on screen.
- The pointer returns to normal as soon as the recording ends: stopped, cancelled, failed, or the app quits.
- The same for a recording started by the hotkey and one started by voice.
- The sign follows the pointer on every screen.

## Capabilities

### New Capabilities

- `dictation-pointer`: the mouse pointer shows that a recording runs.

### Modified Capabilities

None.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: the pointer sign during a recording, in push-to-talk and voice control, and its return to normal.
- Acceptance bar: during every recording the pointer shows the dictation sign over any app; it changes back within a moment of the recording's end; it is normal after the app quits during a recording; clicks and pointer movement work as usual.

## Out of scope

- A pointer sign while processing or while waiting for focus.
- User-chosen pointer images.
- A sign that moves with the voice level.
- A sign at the text caret instead of the pointer.

## Release

MINOR (new capability): 0.20.0.

## Impact

- macOS lets an app set the pointer only over its own windows; a change over every app may need a small sign that follows the pointer, or another approach. Design decides.
- A new driven port or a new signal of the visual indicator, with a macOS adapter.
- `docs/ubiquitous-language.md`: a term for the pointer sign.
- Manual run by the user (overlay, recording).
