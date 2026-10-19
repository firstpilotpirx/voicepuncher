## Why

While a recording runs, a microphone dictation sign is shown next to the mouse pointer. The user sees it sitting too far from the pointer: too far down and to the right. The vertical gap is the worst part — the sign drops too far below the pointer, so it no longer reads as attached to it.

## What Changes

- The dictation sign next to the pointer is drawn closer to the pointer: both the horizontal (right) and the vertical (down) offset are reduced to roughly half of today's values, the vertical offset at least by half.

Current stage: PoC. Target stage: PoC.
Scope: the offset of the dictation sign from the pointer.
Acceptance bar: the sign's offset from the pointer is about half of the current one on both axes (tests pin the new offsets), and the user confirms in a manual run that it sits close to the pointer.
Out of scope: the sign's size, look, and when it is shown or hidden.

## Capabilities

### Modified Capabilities

- `dictation-pointer`: the dictation sign's position relative to the pointer moves closer — about half of today's right and down offset.

## Impact

- The overlay code that places the dictation sign, and its tests.
- Manual run by the user (overlay changed).

## Decisions

- New offsets: 8 points right and 6 points down from the pointer's tip (today 16 and 16) — half on the right, a little more than half down, where the complaint is.
- Capability: `dictation-pointer` (the spec on `main` for the sign next to the pointer).
- Target version: 0.22.1 (PATCH on 0.22.0).

## Release

SemVer: PATCH — a visual adjustment of existing behavior.
Target version: 0.22.1.
