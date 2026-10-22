# Acceptance run

A manual end-to-end pass of the installed app, the way a new user meets it. Claude runs it through computer use (change `agent-manual-acceptance-run`); anyone can run it by hand. Each item names the capability and requirement it checks (`openspec/specs/<capability>/spec.md`); when a requirement changes, update its item.

Record each item as **pass**, **fail** (with a one-line observation and a screenshot) or **blocked** (could not run, with the reason). Copy this list into `docs/acceptance-runs/<date>.md` for a run's report.

## Before the run

- Install the disk image of the version under test: the GitHub Release's `.dmg`, or `just dmg` on a clean checkout of `origin/main`.
- Stop the services the scripts run (`just stop-model-servers`) and quit any running voicepuncher, so the installed app starts its own.
- Move `~/.voicepuncher/config.toml` and `~/.voicepuncher/history` aside (moved, never deleted), and copy an installed `/Applications/voicepuncher.app` aside. Models stay in place: other checkouts' checks read them.
- Apps Claude needs access to: Finder, voicepuncher, System Settings, TextEdit.

## 1. Install

| # | Item | Checks |
|---|---|---|
| 1.1 | Opening the disk image shows the app and an Applications shortcut | app-distribution: The release is one disk image |
| 1.2 | Dragging the app to Applications copies it; it starts from there | app-distribution: The first launch is documented |
| 1.3 | The app shows no Dock icon and an icon appears in the menu bar | menu-bar-presence: The app has no Dock icon; The menu bar shows the app from launch |
| 1.4 | A first launch creates `~/.voicepuncher/config.toml` with the defaults | settings-window: A first launch creates the configuration file with the defaults |
| 1.5 | Nothing is asked in a terminal | push-to-talk: Nothing is asked in a terminal |

## 2. Permissions

| # | Item | Checks |
|---|---|---|
| 2.1 | The permissions window lists Microphone, Accessibility, Input Monitoring, each with its status and a remedy button | permissions-gate: Each permission is shown with its status and its remedy |
| 2.2 | A permission never asked differs from a denied one | permissions-gate: Not asked yet differs from denied |
| 2.3 | The window names the app that must be granted | permissions-gate: The window names who must be granted |
| 2.4 | The menu says it waits for permissions | menu-bar-presence: The menu shows the wait for permissions |
| 2.5 | Granting a permission updates the window without a restart | permissions-gate: The window is current and can be opened at any time |
| 2.6 | With all three granted, dictation becomes possible | permissions-gate: Dictation starts only with every permission granted |
| 2.7 | Permissions opens the window again from the menu | menu-bar-presence: Permissions open from the menu |

## 3. Models and services

| # | Item | Checks |
|---|---|---|
| 3.1 | With a models folder lacking models, the missing models are listed with sizes before anything downloads | model-download: Missing models are listed before anything downloads |
| 3.2 | The menu says it waits for models; Download models opens the download | menu-bar-presence: The menu shows the wait for models; Download models opens from the menu |
| 3.3 | The download fetches, verifies and places each file, showing progress | model-download: Download fetches, verifies and places each file |
| 3.4 | The services start once the last file is present | model-download: The services start when the last file is present |
| 3.5 | The menu shows the services starting, then lists each with its state | menu-bar-presence: The menu shows the services starting; The menu lists the services |
| 3.6 | The services run from the bundle's own engines (log names the engine) | local-services: A bundled app starts its services from its own engines; The engine in use is logged |

## 4. Menu bar

| # | Item | Checks |
|---|---|---|
| 4.1 | The menu tells the state in words | menu-bar-presence: The menu tells the state in words |
| 4.2 | The icon changes while dictating | menu-bar-presence: The icon mirrors the dictation |
| 4.3 | The icon shows voice control on and off | menu-bar-presence: The icon shows voice control |
| 4.4 | Logs opens the logs folder in Finder | menu-bar-presence: The logs folder opens from the menu |
| 4.5 | Settings opens the settings window | menu-bar-presence: Settings open the settings window from the menu |
| 4.6 | Every other item and sub-item does what its label says | menu-bar-presence |

## 5. Settings

| # | Item | Checks |
|---|---|---|
| 5.1 | One settings window opens; opening it again focuses the same window | settings-window: One settings window opens from the menu |
| 5.2 | Every option has a control | settings-window: Every option has a control |
| 5.3 | A changed setting applies on save without a relaunch | settings-window: Saving applies without a relaunch |
| 5.4 | An invalid value is refused with a message | settings-window: An invalid value is refused |
| 5.5 | Revert drops unsaved edits | settings-window: Revert and unsaved edits |
| 5.6 | Sounds can be played before saving | settings-window: Sounds can be heard before saving |
| 5.7 | The Services tab shows the models and the services | settings-window: The Services tab holds the models and the services |
| 5.8 | A saved setting survives quit and relaunch | settings-window: The user's configuration file is preserved |

## 6. Launch at login

| # | Item | Checks |
|---|---|---|
| 6.1 | The launch at login offer is made once | launch-at-login: The offer is made once |
| 6.2 | The switch shows the system's state | launch-at-login: The switch shows the system's state |
| 6.3 | Switching on registers the app in System Settings › Login Items; switching off removes it | launch-at-login: Switching on registers, switching off unregisters |

## 7. Dictation

| # | Item | Checks |
|---|---|---|
| 7.1 | Hold the hotkey in TextEdit, play a phrase with `say`, release: the text is pasted | push-to-talk; text-insertion: Text is pasted at once unless no text input is surely focused |
| 7.2 | The clipboard holds what it held before the paste | text-insertion: The paste keeps the user's clipboard |
| 7.3 | The hotkey works again after the paste | push-to-talk: The hotkey keeps working after a paste |
| 7.4 | Without a focused text input the text waits and is pasted when a text input gets focus | text-insertion: Held-back text is pasted when a text input gets focus |

## 8. Voice commands

| # | Item | Checks |
|---|---|---|
| 8.1 | "start dictation", a phrase, "stop dictation": the phrase is pasted | voice-control: "start dictation" / "stop dictation" |
| 8.2 | "cancel dictation" discards the dictation | voice-control: "cancel dictation" discards a dictation |
| 8.3 | "restart dictation" starts over | voice-control: "restart dictation" starts the dictation over |
| 8.4 | "press enter" presses Return | voice-control: "press enter" presses Return |
| 8.5 | "clear all text" and "clear text" erase the focused text | voice-control: "clear all text" erases the focused text |
| 8.6 | "paste text" pastes the clipboard | voice-control: "paste text" pastes the clipboard |
| 8.7 | A short form of each command works | voice-control: Every command has a one-word short form |
| 8.8 | A hands-free session (start, phrase, stop, press enter) runs as said | voice-control: A whole hands-free session runs as said |
| 8.9 | The hotkey ends a dictation started by voice, and a voice command ends one the hotkey started | voice-control: The hotkey ends a dictation started by voice; Voice commands end a dictation the hotkey started |

## 9. Quit and relaunch

| # | Item | Checks |
|---|---|---|
| 9.1 | Quit ends the app and stops every service it started | menu-bar-presence: Quit ends the application; local-services: Quit stops every owned service |
| 9.2 | A relaunch keeps settings and needs no new permission | settings-window; app-distribution: The signature keeps the identity across updates |

## After the run

Quit the app, switch launch at login off if the run left it on, move the user's files and app back, and start the user's services again.
