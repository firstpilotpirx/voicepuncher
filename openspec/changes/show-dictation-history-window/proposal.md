## Why

When a dictated text gets lost (the paste went to the wrong window, the clipboard was overwritten, the target app dropped it), the user has no quick way to get it back from the menu bar app. The app already keeps past dictations (the dictation archive), but the GUI shows none of them. The user wants a separate page in the GUI listing recent dictations so the lost text can be found and copied.

## What Changes

- The menu bar app gets a separate window (page), opened from its menu, that shows the history of previous dictations as a table, newest first.
- Each row shows the dictated text and when it was dictated; the user can copy a row's text to the clipboard.
- The window shows only the most recent entries (a few hundred, e.g. the last 300), not the whole archive.

Current stage: PoC. Target stage: PoC.
Scope: a read-only history window in the menu bar app, fed from the existing dictation archive, with copy of one entry.
Acceptance bar: after several dictations, opening the history window from the menu lists them newest first with their time, at most the configured limit, and copying a row puts exactly its text on the clipboard.
Out of scope: editing or deleting entries, full-text search across the whole archive, voice command history, changing what the archive stores.

## Capabilities

### New Capabilities

- `dictation-history-window`: a GUI page listing the most recent dictations, newest first, with copy of an entry's text.

## Impact

- `app/`: the menu bar adapter (new menu item and window), a port reading the latest N entries of the dictation archive.
- `docs/ubiquitous-language.md`: the term for the history window.

## Open points

- Limit of entries shown: proposed 300, fixed; or a `[history]` setting in the configuration file.
- Copy interaction: proposed a copy button per row plus copy on double-click; select-and-Cmd+C as an alternative.
- Whether the window refreshes live while open after a new dictation: proposed yes.
- Whether to add a simple filter box over the shown entries: proposed no for now.

## Release

SemVer: MINOR — new capability.
Target version: 0.40.0.
