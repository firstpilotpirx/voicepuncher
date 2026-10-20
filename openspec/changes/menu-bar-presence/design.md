# Design

## Context

The overlay adapter (`adapters/secondary/visualfeedback/macos/`, `objc2-app-kit` 0.3) is the only AppKit user; it creates its windows on the main thread with a `MainThreadMarker` and dispatches updates to the main queue; `screen_watcher.rs` shows `define_class!` for an Objective-C delegate. `PointerSignIndicator` is a `VisualIndicator` decorator. `restartable-app-runtime` gives `RuntimeState` and `RuntimeObserver`. `main.rs` enters `NSApplication.run()`.

## Decisions

**D1 — A `VisualIndicator` decorator feeds the icon.** The status item learns dictation states by decorating the indicator chain, exactly as the pointer sign does, instead of a new port or pipeline hook. The pipeline stays unchanged; the mapping is a pure, unit-tested function. Alternative: the pipeline publishes a state enum on a channel — rejected, the overlay signals already carry every transition.

**D2 — Runtime states through the observer.** Starting, Restarting and Failed come from `RuntimeObserver`; the icon shows a runtime state only while no dictation is in progress (a dictation cannot run in those states anyway). Calls arrive on the runtime thread and are dispatched to the main queue.

**D3 — Primary adapter in `adapters/primary/menubar/macos/`.** The menu drives the application (Quit, Settings); displaying the icon is the smaller half. Actions go through a small `MenuActions` trait implemented by the runtime wiring (`quit`, `open_settings`, `open_logs`), so the adapter's unit tests use a mock.

**D4 — SF Symbols, template images, one non-template recording badge.** Template images follow the menu bar appearance for free; the recording state needs colour to be seen at a glance, so it is the one non-template image, per the mockups. No bitmap assets.

**D5 — Menu rebuilt on open.** `menuNeedsUpdate:` sets the title and status line from the current state right before the menu shows; the icon updates on every state change. No timers except the processing animation.

**D6 — Accessory activation policy in code.** `LSUIElement` hides the Dock icon for the bundle; `setActivationPolicy(Accessory)` does the same for the unbundled developer binary, so both look alike.

**D7 — Quit cancels, never waits.** Quit is the user's decision; a recording is cancelled with the cancel sound (the pipeline's cancel path) and the runtime stops; the process exits 0. A restart waits for a dictation (`restartable-app-runtime`), a quit does not.

## Risks / Trade-offs

- [The status item is released by AppKit when no Rust value keeps it] → one long-lived struct owns the item, the menu, the target and the delegate; it is leaked on purpose at quit.
- [Symbol names missing on the minimum macOS (15)] → every symbol named in the design spec exists since macOS 11–14; the adapter falls back to `mic` when `imageWithSystemSymbolName` returns nil.
- [The icon animation wakes the main thread while processing] → 400 ms frames only during Processing, none with Reduce Motion.

## Test levels

| Scenario | Level |
|---|---|
| Signal → icon state, icon state → symbol name/template, runtime state → status line, hotkey → human name | unit tests in `mapping.rs` |
| The icon mirrors the dictation (each signal), a command sign leaves it | component `menu_bar_presence.feature` (mock status item surface, real decorator) |
| The menu bar shows the app from launch; starting, restarting, failed states | component `menu_bar_presence.feature` (observer driven) |
| Quit stops the runtime, cancels a recording, exits 0 | component `menu_bar_presence.feature` (mock actions) + manual run |
| Open Logs Folder creates and opens the folder; Settings opens the file | adapter integration `menubar_macos_it.rs` (folder creation; the open calls are smoke-tested with a temp file, skipped without a window server) |
| No Dock icon, pictures match the mockups, light and dark menu bar | manual run by the user |
