## Why

While dictating, a microphone indicator is shown next to the text cursor. The user sees it sitting too far from the cursor: too far down and to the right. The vertical gap is the worst part — the indicator drops too far below the cursor, so it no longer reads as attached to it.

## What Changes

- The microphone indicator next to the text cursor is drawn closer to the cursor: both the horizontal (right) and the vertical (down) offset are reduced to roughly half of today's values, the vertical offset at least by half.

Current stage: PoC. Target stage: PoC.
Scope: the offset of the dictation cursor microphone indicator from the text cursor.
Acceptance bar: the indicator's offset from the cursor is about half of the current one on both axes (tests pin the new offsets), and the user confirms in a manual run that it sits close to the cursor.
Out of scope: the indicator's size, look, and when it is shown or hidden.

## Capabilities

### Modified Capabilities

- `dictation-cursor`: the indicator's position relative to the text cursor moves closer — about half of today's right and down offset.

## Impact

- The overlay code that places the dictation cursor indicator, and its tests.
- Manual run by the user (overlay changed).

## Open points

- Exact new offsets: proposed half of the current right and down offsets; the user said "about half", so the design may pick a slightly smaller vertical offset if half still looks low.
- Capability name: proposed `dictation-cursor` (from the show-dictation-cursor change); design corrects it to the actual spec name.
- Target version: proposed 0.21.1 (PATCH on 0.21.0).

## Release

SemVer: PATCH — a visual adjustment of existing behavior.
Target version: 0.21.1.
