# Design

## Context

The overlay adapter (`adapters/secondary/visualfeedback/macos/`, `objc2-app-kit` 0.3) is the only AppKit user; it creates its windows on the main thread with a `MainThreadMarker` and dispatches updates to the main queue; `screen_watcher.rs` shows `define_class!` for an Objective-C delegate. `PointerSignIndicator` is a `VisualIndicator` decorator. `restartable-app-runtime` gives `RuntimeState` and `RuntimeObserver`. `main.rs` enters `NSApplication.run()`.

## Decisions

**D1 — A `VisualIndicator` decorator feeds the icon.** The status item learns dictation states by decorating the indicator chain, exactly as the pointer sign does, instead of a new port or pipeline hook. The pipeline stays unchanged; the mapping is a pure, unit-tested function. Alternative: the pipeline publishes a state enum on a channel — rejected, the overlay signals already carry every transition.

**D2 — Runtime states through the observer.** Starting, Restarting and Failed come from `RuntimeObserver`; the icon shows a runtime state only while no dictation is in progress (a dictation cannot run in those states anyway). Calls arrive on the runtime thread and are dispatched to the main queue.

**D2a — A failed start still exits.** The specs of speech recognition, text refinement and voice control require the app to exit when a service does not answer at launch; the user kept that. The failed icon and status line show until the exit; staying in the menu bar while a service is missing belongs to `local-services-runtime`.

**D3 — Primary adapter in `adapters/primary/menubar/macos/`.** The menu drives the application (Quit, Settings); displaying the icon is the smaller half. Actions go through a small `MenuActions` trait implemented by the runtime wiring (`quit`, `open_settings`, `open_logs`), so the adapter's unit tests use a mock.

**D4 — SF Symbols, template images, one non-template recording badge.** Template images follow the menu bar appearance for free; the recording state needs colour to be seen at a glance, so it is the one non-template image (a filled microphone with a red dot drawn over it), per the mockups. The processing animation is the `waveform` symbol at the variable values 0.33, 0.66 and 1.0 (SF Symbols' variable rendering), one frame every 400 ms, still at 0.66 under Reduce Motion. No bitmap assets; `mockups/render.swift` draws the mockups with the same symbols, so a manual-run screenshot can be compared with them.

**D5 — Menu rebuilt on open.** `menuNeedsUpdate:` sets the title and status line from the current state right before the menu shows; the icon updates on every state change. No timers except the processing animation.

**D6 — Accessory activation policy in code.** `LSUIElement` hides the Dock icon for the bundle; `setActivationPolicy(Accessory)` does the same for the unbundled developer binary, so both look alike.

**D7 — Quit cancels, never waits.** Quit is the user's decision; a recording is cancelled with the cancel sound (the pipeline's cancel path) and the runtime stops; the process exits 0. Between the stop and the exit the runtime reports Stopped, which shows the waiting icon and the status line "Quitting…". A restart waits for a dictation (`restartable-app-runtime`), a quit does not.

**D8 — The status item is a `VisualIndicator` decorator and a `RuntimeObserver` sharing one state cell.** Both feed a single icon state behind a lock (dictation stage, runtime state, voice control listening, commands unavailable); a pure function picks the icon and the status line from the cell, so the precedence (a dictation outranks the commands-unavailable sign, which outranks listening, which outranks ready; a runtime state shows only while no dictation runs) lives in one tested place.

## Dependencies

`restartable-app-runtime` supplies `Runtime::stop()` (Quit), `RuntimeState` and `RuntimeObserver` (the waiting, ready and failed icons and status lines) and the shared overlay wrapper the decorator is installed in; this change waits for its tasks and stacks on its branch.

## Risks / Trade-offs

- [The status item is released by AppKit when no Rust value keeps it] → one long-lived struct owns the item, the menu, the target and the delegate; it is leaked on purpose at quit.
- [Symbol names missing on the minimum macOS (15)] → every symbol named in the design spec exists since macOS 11–14; the adapter falls back to `mic` when `imageWithSystemSymbolName` returns nil.
- [The icon animation wakes the main thread while processing] → 400 ms frames only during Processing, none with Reduce Motion.

## Test levels

| Scenario | Level |
|---|---|
| Signal and runtime state → icon state (precedence), icon state → symbol name, template flag, red dot and animated flag (the frame values and the 400 ms step are constants, checked by the manual run); state → status line; hotkey → human name | unit tests in `mapping.rs` |
| The status item appears while the app starts | component `menu_bar_presence.feature` (observer driven, status item double) |
| The ready icon shows once the runtime is ready | component `menu_bar_presence.feature` |
| A restart shows the waiting icon and ready again afterwards | component `menu_bar_presence.feature` |
| A failed runtime shows the failed icon until the app exits | component `menu_bar_presence.feature` |
| A recording shows the recording icon | component `menu_bar_presence.feature` (real decorator, status item double) |
| The icon follows every stage of a dictation | component `menu_bar_presence.feature` |
| A failure notice shows the warning icon until it hides | component `menu_bar_presence.feature` |
| A command sign leaves the icon as it was | component `menu_bar_presence.feature` |
| Voice control listening shows the listening icon | component `menu_bar_presence.feature` |
| Commands unavailable shows the warning badge on the ear | component `menu_bar_presence.feature` |
| A dictation outranks the commands-unavailable sign | component `menu_bar_presence.feature` |
| The ready status line names the hotkey | component `menu_bar_presence.feature` (menu opened through the double) |
| The ready status line in Toggle mode says press | component `menu_bar_presence.feature` |
| The recording status line says how to stop | component `menu_bar_presence.feature` |
| The failed status line gives the reason | component `menu_bar_presence.feature` |
| The status line reflects the state at the moment the menu opens | component `menu_bar_presence.feature` |
| Every hotkey has a human name | component `menu_bar_presence.feature` (scenario outline over the hotkeys) + unit tests in `mapping.rs` |
| Quit while idle ends the process cleanly | component `menu_bar_presence.feature` (mock actions) + manual run |
| Quit during a recording cancels it | component `menu_bar_presence.feature` (recording overlay double) + manual run |
| Open Logs Folder shows the folder | adapter integration `menubar_macos_it.rs` (temp home; the call that opens Finder runs only with `VOICEPUNCHER_WINDOWS_IT=1`, which no recipe sets, so the manual run covers it) |
| Open Logs Folder creates a missing folder | adapter integration `menubar_macos_it.rs` |
| Settings opens the configuration file | adapter integration `menubar_macos_it.rs` (temp file; runs only with `VOICEPUNCHER_WINDOWS_IT=1`, which no recipe sets, so the manual run covers it) |
| The bare binary shows no Dock icon | manual run by the user |
| Pictures match the mockups, light and dark menu bar, the animation | manual run by the user |
