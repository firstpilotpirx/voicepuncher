## 1. Reading the dictation archive

- [x] 1.1 Red: unit tests of `HistoryRow::from_texts` (pasted text, raw transcript, neither, whitespace only) in `app/src/core/application/dictation_history.rs`
- [x] 1.2 Green: the `DictationHistory` port, `HistoryRow` and `HistoryList` in `app/src/core/application/dictation_history.rs`
- [x] 1.3 Red: adapter integration tests of the reader (newest first, limit, `.partial` folders skipped silently, damaged and unknown-format entries skipped with a warning, missing folder) in `app/tests/it/archive_history.rs`
- [x] 1.4 Green: the filesystem reader in `app/src/adapters/secondary/archiving/filesystem/history.rs`
- [x] 1.5 `just test` and commit

## 2. The history window model

- [x] 2.1 Red: component scenarios of the Rules "The history window lists the latest dictations newest first", "A row's text is copied to the clipboard" (copy), "The open window shows a new dictation at once", "The history window explains an empty list" and "An unreadable entry does not stop the history window" in `app/tests/features/component/dictation_history_window.feature`, steps in `app/tests/steps/dictation_history_window/`
- [x] 2.2 Red: unit tests of the local time label in `app/src/adapters/primary/history/`
- [x] 2.3 Green: `HistoryView` (rows, time label, empty-list line, copy, refresh) in `app/src/adapters/primary/history/mod.rs`
- [x] 2.4 Add history window and history row to `docs/ubiquitous-language.md`
- [x] 2.5 `just test` and commit

## 3. The window and the menu item

- [x] 3.1 Red: component scenario "Dictation history opens the history window" in `app/tests/features/component/menu_bar_presence.feature`, steps in `app/tests/steps/menu_bar_presence/`
- [x] 3.2 Green: `MenuActions::open_history` and the menu item "Dictation history…" in `app/src/adapters/primary/menubar/macos/`
- [x] 3.3 Red: main-thread tests "Double-click copies the row", "Choosing it again keeps one window" and the refresh check loop in `app/tests/main_thread/history_macos.rs`
- [x] 3.4 Green: the AppKit window (table, Copy button, double-click, one-second check while visible, `bring_to_front`) in `app/src/adapters/primary/history/macos/`
- [x] 3.5 Green: wire the window with the saved configuration's archive settings in `app/src/menubar_app.rs`
- [x] 3.6 `just test` and commit

## 4. Release

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.41.0
- [x] 4.2 `just check` once
- [x] 4.3 Manual checks: none of the AGENTS.md manual-run areas changed; the agent opens the history window in the built app, dictates nothing new, and copies a row (visual check)
