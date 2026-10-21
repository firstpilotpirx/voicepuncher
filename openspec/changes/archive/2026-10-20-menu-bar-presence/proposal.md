## Why

voicepuncher runs as a console program: after launch it prints log lines to the terminal and shows nothing else until the user holds the hotkey; the only way to quit is Ctrl-C in that terminal. The user wants a macOS application they install and forget: a menu bar (tray) icon that shows the app is running and what it is doing, and a menu to reach the settings, the logs and Quit. With `LSUIElement` in the bundle (`app-bundle-and-dmg`) the app has no Dock icon and no application menu, so the menu bar item is its only presence and the only way to quit it.

The overlay already knows every state of a dictation (`VisualIndicator`: recording, recognizing speech, refining text, waiting for focus, pasted, failure notice, command sign, commands unavailable); the runtime (`restartable-app-runtime`) knows whether the pipeline is starting, ready, restarting or failed. The menu bar icon mirrors both, so the user sees at a glance that the hotkey will work.

## What Changes

- At launch the app puts an item in the menu bar (an `NSStatusItem` of the system status bar, square length). Its icon shows the state; the exact pictures are fixed by `mockups/design-spec.md` (made with Claude Design before the proposal gate): Ready — the microphone outline (SF Symbol `mic`, a template image, so it follows the light and dark menu bar); Recording — a filled microphone with a red dot (`mic.fill` with a red badge, or `record.circle` in system red, non-template; the mockups decide); Recognizing speech and Refining text — the waveform symbol (`waveform`), animated subtly (three frames or a pulse) when Reduce Motion is off; Waiting for focus — `keyboard`; Voice control listening (the commands-unavailable sign off, listening on) — `ear`; Commands unavailable — `ear.trianglebadge.exclamationmark`; Failure notice — `exclamationmark.triangle` for as long as the overlay shows the notice; Starting, Restarting, Waiting (services, permissions, models of later changes) — `hourglass`; Failed — `exclamationmark.triangle.fill` with the reason in the menu.
- Clicking the icon opens the menu: a disabled title line `voicepuncher 0.27.0`; a disabled status line in plain words (`Ready — hold Right Option to dictate`, `Recording — release Right Option to stop`, `Recognizing speech…`, `Refining text…`, `Waiting for a text field…`, `Voice control: listening`, `Restarting…`, `Starting…`, `Failed: speech recognition service at http://127.0.0.1:8081 does not answer`); a separator; `Settings…` (⌘,) — until `settings-window` lands it opens `~/.voicepuncher/config.toml` in the default text editor; `Open Logs Folder` — opens `~/.voicepuncher/logs/` in Finder (the folder the services' logs live in; `file-logging` adds the app's own log there); a separator; `Quit voicepuncher` (⌘Q).
- Quit stops the runtime (the pipeline finishes nothing: a recording in progress is cancelled with the cancel sound, as Ctrl-C does today), then ends the process with exit code 0. Later changes add their items in fixed places: `Permissions…` under the status line (`permissions-gate-window`), `Services` submenu and `Download models…` (`local-services-runtime`, `model-download`), `Launch at login` lives in the settings window (`launch-at-login`).
- When the app runs from a terminal (developer: `cargo run`), the menu bar item appears the same; the activation policy is set to Accessory so no Dock icon appears for the unbundled binary either.
- The hotkey name in the status line is the human name of the configured key (`Right Option`, `Right Command`, `Fn`, `Caps Lock`, `Option+E`, `Cmd+E`, `F13`…), the same names the settings window will use.
- Nothing changes in the dictation itself, the overlay, the sounds or voice control.

Current stage: PoC. Target stage: Prototype (the app is visible and quittable without a terminal).
Scope: the status item, its icons and menu, the mapping from overlay signals and runtime states to icon and status line, Quit, Settings (file open until the window exists), Open Logs Folder, mockups, docs and glossary, the GUI row of the tech matrix.
Acceptance bar: the icon appears within 1 s of launch and shows every state above during a dictation and a voice command; the menu shows the right status line; Quit ends the process cleanly (log `runtime: stopped`, exit code 0); the pictures of the manual run match the approved mockups in the light and the dark menu bar; the user pastes a healthy log.
Out of scope: a settings window (`settings-window`), a permissions window, services and model states beyond the `hourglass` placeholder (they arrive with their changes), a Dock icon (never), notifications (`NSUserNotification`/`UNUserNotificationCenter`, not needed).

## Capabilities

### New Capabilities

- `menu-bar-presence`: the application's presence in the menu bar: the icon states, the menu with the status line, Settings, Open Logs Folder and Quit; the activation policy without a Dock icon.

### Modified Capabilities

- none.

## Requirements to specify

1. The menu bar shows the app from launch: a status item with the Ready icon appears when the runtime is ready; while it starts or restarts it shows the waiting icon; when it failed, the warning icon.
2. The icon mirrors the dictation: recording, recognizing, refining, waiting for focus, pasted (back to ready), failure notice (warning while the notice shows) each have their icon; a voice command sign leaves the icon as it was; listening on and commands unavailable have theirs.
3. The menu tells the state in words: the status line names the state and, when ready, the hotkey in human words.
4. Quit ends the application: choosing Quit (or ⌘Q with the menu open) stops the runtime and the process exits with code 0; a recording in progress is cancelled with the cancel sound.
5. The logs folder opens from the menu: choosing Open Logs Folder opens `~/.voicepuncher/logs/` in Finder (creating it when missing).
6. Settings open from the menu: choosing Settings opens the configuration (the file in the default editor in this version).
7. No Dock icon: the app runs with the Accessory activation policy whether bundled or not.

## Implementation guide

1. **Mockups first.** Follow `mockups/design-spec.md`: every icon state in the light and the dark menu bar, the open menu with each status line; PNGs into `mockups/states/`; show them to the user with the proposal at stage 3.
2. **Adapter placement.** `app/src/adapters/primary/menubar/macos/mod.rs` (primary: it drives the app with Quit and Settings; it also displays — like the overlay it uses AppKit through `objc2-app-kit`). Cargo features to add: `NSStatusBar`, `NSStatusItem`, `NSStatusBarButton`, `NSMenu`, `NSMenuItem`, `NSButton` (already present for others), and `objc2-foundation` `NSObject`, `NSString` (present).
3. **Icon states as a decorator.** A `StatusItemIndicator` that implements `VisualIndicator`, wraps the existing indicator chain (`PointerSignIndicator(MacosVisualIndicator, MacosPointerSign)`) the way `PointerSignIndicator` wraps the overlay, forwards every call, and maps each signal to an `IconState` enum (Ready, Recording, Processing, WaitingForFocus, Listening, CommandsUnavailable, Warning, Waiting, Failed). Pure mapping functions (signal → icon state, icon state → SF Symbol name and template flag, runtime state → status line text, HotKey → human name) live in `menubar/macos/mapping.rs` and are unit-tested. The decorator is installed by the runtime's wiring (`restartable-app-runtime`), inside the shared overlay wrapper, so it survives restarts.
4. **Runtime states.** A `RuntimeObserver` (from `restartable-app-runtime`) implemented by the status item: Starting and Restarting → Waiting icon and status line; Ready → Ready icon unless a dictation is in progress; Failed → Failed icon and the reason in the status line. Observer calls arrive on the runtime thread: dispatch to the main thread with the same main-queue helper the overlay uses.
5. **The status item itself.** Created in `main` (after `NSApplication::sharedApplication(mtm)`, before `run()`) with the `MainThreadMarker`: `NSStatusBar::systemStatusBar().statusItemWithLength(NSSquareStatusItemLength)`; its button gets an `NSImage::imageWithSystemSymbolName_accessibilityDescription` image with `setTemplate(true)` for template states and a symbol configuration of point size 16; the menu is an `NSMenu` whose items have a target and action on an Objective-C object declared with `define_class!` (as `screen_watcher.rs` does), with methods `settings:`, `openLogs:`, `quit:`; the menu's delegate `menuNeedsUpdate:` refreshes the status line and the title before the menu opens. `NSApp.setActivationPolicy(NSApplicationActivationPolicyAccessory)` in `main`.
6. **Actions.** Quit: calls `Runtime::stop()` (cancels the recording through the pipeline's existing cancel path), then `std::process::exit(0)`, so the exit code is explicit and testable. Settings: `NSWorkspace::sharedWorkspace().openURL(file URL of config.toml)`; when no editor is bound to `.toml`, open with TextEdit (`openURLs_withApplicationAtURL`). Open Logs Folder: create `~/.voicepuncher/logs` when missing, then `NSWorkspace::selectFile_inFileViewerRootedAtPath` on the folder.
7. **Animation.** The Processing icon alternates between two or three symbol frames every 400 ms on the main queue when Reduce Motion is off (read from `NSWorkspace` as the overlay does); still when on. Recording shows the red badge without animation.
8. **Docs and glossary.** `docs/ubiquitous-language.md`: Status item, Icon state, Status line; `docs/project-map.md`: `adapters/primary/menubar/`; `docs/architecture/overview.md` § Technology Choices: GUI toolkit row — AppKit through `objc2-app-kit` (menu bar item, menus; windows in later changes), no SwiftUI, no cross-platform toolkit; `README.md` § How to use: the menu bar item and its menu. Tech matrix (agent-kit PR, `tech-matrix-rust.md`, section "macOS Desktop"): GUI toolkit — AppKit via `objc2` / `objc2-app-kit` (define_class! for delegates and targets; main-thread marker); menu bar item — `NSStatusItem`; rejected: Tauri, egui, iced, SwiftUI (would need a Swift target).

Pitfalls: every AppKit call must run on the main thread — never touch the status item from the pipeline or runtime thread, dispatch; keep a strong reference to the status item, the menu, the target object and the delegate in a struct that lives until quit, or AppKit releases them and the icon vanishes; `NSStatusItem` images must be about 18 pt for a 22 pt bar, use the symbol configuration, not a scaled bitmap; the title line must be disabled (`setEnabled(false)`) and not a header style, so it reads as text; a template image must be monochrome, so the red recording badge is a separate non-template image per the mockups.

## Impact

- new `app/src/adapters/primary/menubar/macos/{mod.rs,mapping.rs}`, `app/src/adapters/primary/mod.rs`
- `app/src/main.rs` and `app/src/runtime/` (wiring of the decorator, observer registration, Quit), `app/Cargo.toml` (features), `Cargo.lock`
- new `app/tests/features/component/menu_bar_presence.feature`, `app/tests/steps/menu_bar_presence.rs`, unit tests in `mapping.rs`, `app/tests/menubar_macos_it.rs` (the status item is created and removed without a crash, on the main thread; the tests that open Finder or an editor run only with `VOICEPUNCHER_WINDOWS_IT=1`)
- `docs/ubiquitous-language.md`, `docs/project-map.md`, `docs/architecture/overview.md`, `README.md`; agent-kit tech matrix (separate PR)
- `mockups/design-spec.md`, `mockups/states/*.png`
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user (overlay area): every state in the light and the dark menu bar, a voice command, Quit; the log pasted.

## Open points

- Recording icon: red-badged microphone (coloured, non-template) or a filled template microphone that stays monochrome: proposed the red badge (the mockups show both; the user picks at the gate).
- Whether the status line shows the hotkey while recording ("Recording — release Right Option to stop"): proposed yes in Hold mode, "Recording — press Right Option to stop" in Toggle mode.
- Settings item before the window exists: open the file in the default editor (proposed) or hide the item until `settings-window`.

## Release

SemVer: MINOR — new user-visible presence and a new capability; pre-1.0.
Target version: 0.29.0 (corrected at merge with `kit version-check`: main took 0.27.0 and 0.28.0).
