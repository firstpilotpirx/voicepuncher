## Why

When a dictated text gets lost (the paste went to the wrong window, the clipboard was overwritten, the target app dropped it), the user has no quick way to get it back from the menu bar app. The app already keeps past dictations (the dictation archive), but the GUI shows none of them. The user wants a separate page in the GUI listing recent dictations so the lost text can be found and copied.

## What Changes

- The menu gets an item that opens the history window (one window; choosing the item again brings it to the front).
- The window lists the latest dictations that have text, newest first, at most 300: each row shows when it was dictated (local time) and its text — the pasted text, else the raw transcript. Dictations without text are not listed.
- A copy button on each row, and a double-click on a row, put exactly that row's text on the clipboard.
- While open, the window adds a new dictation as soon as it is archived.
- With the archive off or empty, the window says so in one line instead of an empty table.
- An unreadable entry in the archive folder is skipped; the window still opens.

Current stage: PoC. Target stage: PoC.
Scope: a read-only history window in the menu bar app, fed from the existing dictation archive, with copy of one entry.
Acceptance bar: after several dictations, opening the history window from the menu lists them newest first with their time, at most 300, and copying a row puts exactly its text on the clipboard.
Out of scope: editing or deleting entries, search or filter, voice command history, changing what the archive stores, a setting for the limit.

## Capabilities

### New Capabilities

- `dictation-history-window`: a GUI window listing the most recent dictations, newest first, with copy of an entry's text.

### Modified Capabilities

- `menu-bar-presence`: the menu gains the item that opens the history window.

## Impact

- `app/`: the menu bar adapter (new menu item), a new primary adapter for the window, a port reading the latest entries of the dictation archive and its filesystem adapter.
- `docs/ubiquitous-language.md`: the terms history window and history row.

## Release

SemVer: MINOR — new capability.
Target version: 0.41.0.
