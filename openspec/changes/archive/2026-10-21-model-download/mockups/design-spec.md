# Model download window and Services tab — design spec

What to make with Claude Design before the proposal gate (stage 2); the user approves at stage 3 with the proposal; the implementation is done when screenshots of the real window and tab match the pictures.

## How to make the pictures

Call the `Artifact` tool with `action: "quickstart"` and `intent: "design"`, follow the Design type it names, and draw the window and the tab as macOS 26 windows at 2x (SF Pro, system controls, light and dark). Export PNGs into `mockups/states/`. Without Claude Design: static HTML in `mockups/model-download.html?state=…&bg=…`, screenshots with the built-in browser.

## 1. Download window

- Title `Download speech models`; 520×420 pt; not resizable.
- Intro (13 pt): `voicepuncher works offline. It needs these files once, 3.3 GB in total. They go to ~/.voicepuncher/models.`
- Table (6 rows, system table look, alternating rows off): columns File (name, monospace 12 pt), Purpose (secondary), Size (`547 MB`), Status (text or a small determinate progress bar with the percent).
- Under the table: total bar full width with `1.2 GB of 3.3 GB · 18 MB/s · 2 min left` beneath; the free-space line `Free space: 120 GB` (red form `Not enough space: needs 3.3 GB, 1.1 GB free`).
- Bottom bar: left `Open Folder`, `Quit`; right `Download` (default; becomes `Cancel` while running), `Done` (enabled only when all present).
- Row status strings: `Present ✓`, `Missing`, `Downloading 42 %`, `Verifying…`, `Failed: no internet connection`, `Failed: checksum mismatch — downloading again`, `Cancelled`.

## 2. Services tab (Settings window)

- Toolbar icon `server.rack`, label `Services`.
- Rows: Models folder (path control + `Choose…`), Models (`6 of 6 present` + `Download missing…` + `Show in Finder` + `Check files`), Dictation copies (number 1–8), Command copies (number 1–8), Manage the services (switch, help `Off: start the services yourself with just start-model-servers`).
- Group `Status`: a four-row read-only list — Dictation (3 copies), Commands (3 copies), Text refinement, Command spotting — each with a coloured dot and state text (`Ready`, `Starting`, `Restarting (try 3)`, `Failed — see log`, `Adopted`, `Stopped outside the app`) and a `Restart` button for owned services.

## 3. Pictures to export

`states/01-missing--{light,dark}.png` (all missing, free space fine), `states/02-downloading--{light,dark}.png` (file 2 at 42 %), `states/03-verifying--light.png`, `states/04-failed-no-internet--light.png`, `states/05-no-space--light.png`, `states/06-done--{light,dark}.png`, `states/07-services-tab--{light,dark}.png`, `model-download-overview.png`.

## 4. Acceptance

The real window in states 01, 02 and 06 and the Services tab (light and dark) match the pictures: same columns, same buttons, same wording.
