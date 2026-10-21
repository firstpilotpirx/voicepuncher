# Tasks

## 1. Mapping: icon state and status line

- [x] 1.1 Red: unit tests in `app/src/adapters/primary/menubar/macos/mapping.rs` — each overlay signal and runtime state sets the state cell; the icon state follows the precedence of design D8 (dictation > commands unavailable > listening > ready; runtime waiting and failed only while no dictation runs; a command sign changes nothing); each icon state names its symbol, template flag, red dot and animated flag; run them and see them fail
- [x] 1.2 Green: `IconState`, the state cell and the pure mapping functions in `mapping.rs`; `app/src/adapters/primary/menubar/{mod.rs,macos/mod.rs}` registered in `adapters/primary/mod.rs`
- [x] 1.3 Red: unit tests in `mapping.rs` — the status line for every state: ready and recording with the hotkey's human name in Hold and Toggle mode, the processing stages, waiting for a text field, listening, commands unavailable, starting, restarting, failed with the reason; the human name of every `HotKey` variant; run them and see them fail
- [x] 1.4 Green: `status_line` and `hotkey_name` in `mapping.rs`
- [x] 1.5 `just test` and commit

## 2. Component: the icon mirrors the dictation and the runtime

- [x] 2.1 Red: `app/tests/features/component/menu_bar_presence.feature` with the scenarios of "The menu bar shows the app from launch", "The icon mirrors the dictation" and "The icon shows voice control", and `app/tests/steps/menu_bar_presence.rs` driving the decorator and the observer with a status item double that records icon states; register the step module and the world field in `app/tests/cucumber_integration.rs`; run them and see them fail
- [x] 2.2 Green: `StatusItemIndicator` in `menubar/macos/mod.rs` — a `VisualIndicator` decorator over the overlay chain and a `RuntimeObserver`, both updating the state cell and pushing the icon state to a `StatusItemSurface` trait (the double in tests, AppKit in production); 2.1 passes
- [x] 2.3 Red: scenarios of "The menu tells the state in words" in `menu_bar_presence.feature` with steps that open the menu through the surface double and read the status line; run them and see them fail
- [x] 2.4 Green: the menu model (title line, status line, items) is rebuilt from the state cell on every open; 2.3 passes
- [x] 2.5 `just test` and commit

## 3. Component: Quit, Open Logs Folder, Settings

- [x] 3.1 Red: scenarios of "Quit ends the application" in `menu_bar_presence.feature` with a `MenuActions` mock and the recording overlay double; run them and see them fail
- [x] 3.2 Green: `MenuActions` trait (`quit`, `open_settings`, `open_logs`) in `menubar/macos/actions.rs`; Quit calls `quit` which stops the runtime; 3.1 passes
- [x] 3.3 Red: `app/tests/menubar_macos_it.rs` — Open Logs Folder creates a missing folder under a temp home and shows it; Settings opens a temp configuration file; the two that open windows run only with `VOICEPUNCHER_WINDOWS_IT=1`; run them and see them fail
- [x] 3.4 Green: `MacosMenuActions` in `menubar/macos/actions.rs` — `NSWorkspace` file open with the plain text editor fallback, folder creation and `selectFile:inFileViewerRootedAtPath:`; `Runtime::stop()` then `std::process::exit(0)` for Quit (D7); 3.3 passes
- [x] 3.5 `just test` and commit

## 4. AppKit: the status item, its menu and the activation policy

- [x] 4.1 Add the `NSStatusBar`, `NSStatusItem`, `NSStatusBarButton`, `NSButton`, `NSMenu`, `NSMenuItem` features to `objc2-app-kit` in `app/Cargo.toml`; `cargo build` passes
- [x] 4.2 `MacosStatusItem` in `menubar/macos/status_item.rs` implementing `StatusItemSurface`: the system status bar item with a square length, template and badge images from SF Symbols at 16 pt with the `mic` fallback, the waveform frames every 400 ms unless Reduce Motion, the menu with a `define_class!` target for `settings:`, `openLogs:`, `quit:` and a delegate refreshing the title and status line in `menuNeedsUpdate:`, every call dispatched to the main queue; the long-lived owner struct; verified by a `menubar_macos_it.rs` test that creates and removes the status item without a crash, and by the manual run
- [x] 4.3 Wire it in `app/src/main.rs` and the runtime wiring: the status item created with the main thread marker before `run()`, the decorator installed inside the shared overlay wrapper, the observer registered, the menu actions given the runtime after the hang-up reload is set up; `setActivationPolicy(Accessory)` is already set by the overlay's initialization; the main-queue helper moves to `adapters/main_queue.rs`, shared by the overlay and the status item; `cargo run` shows the icon
- [x] 4.4 Add to `app/tests/architecture.rs` that `adapters/primary/menubar` is the only adapter importing `crate::runtime`; run it and see it pass
- [x] 4.5 `just test` and commit

## 5. Docs and glossary

- [x] 5.1 Add "Status item", "Icon state", "Status line" to `docs/ubiquitous-language.md`; `adapters/primary/menubar/` to `docs/project-map.md`; the GUI toolkit row (AppKit through `objc2-app-kit`, menu bar item `NSStatusItem`; rejected Tauri, egui, iced, SwiftUI) to `docs/architecture/overview.md` § Technology Choices; the menu bar item and its menu to `README.md` § How to use
- [x] 5.2 Open the agent-kit PR (https://github.com/firstpilotpirx/agent-kit/pull/228) for `tech-matrix-rust.md` § macOS Desktop (GUI toolkit and menu bar item rows) from `/Users/me/Workspaces/my/agent-kit`
- [x] 5.3 `just test` and commit

## 6. Release

- [x] 6.1 Bump `version` in `app/Cargo.toml` to the next minor version above `main` (0.27.0 at the time of writing; `kit version-check`)
- [x] 6.2 `just check` once and commit
- [x] 6.3 Manual run by the user (overlay area): with `just start-model-servers` running, `cargo run --release -p voicepuncher`; the icon appears within 1 s; a dictation shows recording, waveform, keyboard (focus a button first) and ready again; a voice command leaves it; stop the command spotting service for 10 s — the ear with the badge; open the menu in the ready and the recording state and compare with `mockups/states/10-menu-ready` and `11-menu-recording`; switch the appearance between light and dark; Open Logs Folder, Settings…, then Quit during a recording — the cancel sound, `runtime: stopped` in the log, exit code 0; paste the log
