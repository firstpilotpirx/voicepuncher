# Design

## Context

The dictation archive writes one folder per dictation (`<utc time>/entry.json` + `audio.wav`) through `FileSystemDictationArchive`, pruned to its size limit; nothing reads it back. The menu bar adapter already opens AppKit windows (settings, Permissions, Download) through `MenuActions`, and `adapters::main_queue::on_main` runs work on the main thread. See proposal.md for why.

## Goals / Non-Goals

**Goals:** read the newest entries cheaply; keep the listing rules testable without AppKit; one window instance.

**Non-Goals:** watching the archive with FSEvents; any write to the archive.

## Decisions

1. **A reading port beside the writing one.** `DictationHistory` (core/application) returns the latest rows up to a limit, or that the archive is off. The filesystem adapter lists the folder names (they sort by time), walks them newest first, reads only `entry.json`, and stops once it has 300 rows with text; folders ending in `.partial` (still being written) are skipped silently, as the one-second check would otherwise warn about them again and again; unreadable files and unknown `format` values are skipped with a warning naming the entry. Reading only what is shown keeps the open fast even with tens of thousands of entries. Rejected: an index file kept by the writer — a second format to keep in sync for a PoC.
2. **Row rules in the core.** `HistoryRow::from_texts(started_at, pasted, raw)` picks the pasted text, else the raw transcript, else no row (empty or whitespace-only counts as none). Rejected: deciding in the adapter — the rule is domain behavior and belongs under unit tests.
3. **A window model without AppKit.** `adapters/primary/history/` holds `HistoryView`: rows formatted for display (local time label, text), the empty-list line, `copy(row)` through a copier given to it, and `refresh()` that reloads from the port. The AppKit part (`history/macos/`) only draws the view: an `NSTableView` with a time column, a text column and a Copy button per row, double-click bound to the same copy. This mirrors the settings window and lets the component scenarios drive the model.
4. **Refresh by a one-second check while visible.** While the window is visible, an `on_main_after` loop compares the newest entry folder name with the one shown and refreshes on a change; it stops when the window closes. Rejected: a "kept" hook from the pipeline's archive through the runtime wiring — it crosses the runtime, the reload of the configuration and three modules for a window, and would miss entries another process writes. A second's delay is invisible for this use.
5. **The archive settings come from the saved configuration each time the window opens**, so a folder or on/off changed in the settings window applies at the next open; the refreshes of an open window keep the folder it opened with.
6. **Copy writes the general pasteboard as plain text** from the macOS history adapter; tests hand the view a recording copier, and the AppKit test a named pasteboard.
7. **One window.** The menu action calls `bring_to_front`, which creates the window on first use and afterwards only orders it front, like the Download window.

## Test levels

| Scenario | Level |
|---|---|
| Dictations are listed newest first with their time | component (`dictation_history_window.feature`) |
| A dictation that pasted nothing shows its transcript | component |
| A dictation without text has no row | component |
| Only the newest 300 dictations are listed | component |
| Copy puts exactly the row's text on the clipboard | component |
| Double-click copies the row | adapter integration (main thread, AppKit table) |
| A new dictation appears while the window is open | component (view refresh) + adapter integration (the check loop) |
| The archive is off | component |
| Nothing archived yet | component |
| A damaged entry is skipped | component |
| Dictation history opens the history window | component (`menu_bar_presence.feature`) |
| Choosing it again keeps one window | adapter integration (main thread) |

Unit tests cover `HistoryRow::from_texts` and the time label; adapter integration tests cover the reader (partial folders, unknown format, the limit). Manual run: none of the manual-run areas change; the agent opens the history window in the built app over the existing archive and copies a row (visual check).

## Risks / Trade-offs

- [The one-second check costs a folder listing per second while open] → only while visible, and the check lists only folder names (entries are read only when the newest name changed), so it stays cheap on the main thread.
- [Entry format changes later] → the reader accepts formats 1 and 2 and skips any other with a warning, so a newer writer never breaks the window.
