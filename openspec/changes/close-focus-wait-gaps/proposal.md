# Proposal

## Why

The user wants the text not to be pasted when no text input is focused, but held back until an input gets focus and then pasted at once. The focus wait already does that (`text-insertion`), but it pastes at once whenever it cannot tell what has focus or the focused element is of a generic kind. Browsers and Electron apps report their web content or a group instead of the input, so there the text is still pasted with no input focused and does not reach the place the user meant.

## What Changes

- Find the apps and situations where the text is still pasted with no text input focused; the focus decision log and the known gap of browsers and Electron apps (archived `wait-for-text-focus` design) are the starting point.
- Tell a focused text input from anything else inside web content and other generic elements, and hold the text back there too, until a text input gets focus; then paste it at once, as the focus wait does today.
- Keep pasting at once whenever a text input is focused: a wrong wait costs a paste, so a real input never waits.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-insertion`: the text is held back in web content and other generic elements when no text input is focused there; a scenario reproduces the paste with no input focused.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: the focus decision for browsers, Electron apps and other apps the investigation finds.
- Acceptance bar: in a browser and an Electron app with no text input focused the text waits and is pasted once the user clicks into an input; with an input focused it is pasted at once, as today; the wait timeout and the clipboard fallback work as today.

## Out of scope

- Pasting into the app that was focused when the recording started.
- Inserting text through the Accessibility API instead of Cmd+V.
- Per-app exclusion lists.

## Release

PATCH (bug fix), next patch version after the current one in `app/Cargo.toml` (0.9.0 → 0.9.1 at the time of writing).

## Impact

- The focus probe of the text insertion adapter (macOS Accessibility).
- `text-insertion` slice scenarios and unit tests of the focus decision.
- Manual run by the user (paste, permissions).
