# Menu bar presence — design spec

What to make with Claude Design before the proposal gate (stage 2). The user approves these pictures at stage 3 together with the proposal; the implementation is done when a screenshot of each state in the manual run matches its picture (same symbol, same proportions, same colours on the same menu bar appearance).

## How to make the pictures

Call the `Artifact` tool with `action: "quickstart"` and `intent: "design"`, follow the Design type it names, and draw the states below as a macOS 26 menu bar at 2x. Export PNGs into `mockups/states/`. If Claude Design is not available in the session, build one static HTML page `mockups/menu-bar.html` (system font stack, `?state=` and `?bg=light|dark` query parameters), screenshot it with the built-in browser, as the overlay change did.

## 1. Pictures to export

| File | Shows |
|---|---|
| `states/01-ready--{light,dark}.png` | the right part of the menu bar with the voicepuncher icon in the Ready state between two other menu bar icons (e.g. Wi-Fi, battery) |
| `states/02-recording--{light,dark}.png` | Recording |
| `states/03-processing--{light,dark}.png` | Recognizing speech / Refining text (one picture; show the 3 animation frames side by side in `states/03-processing-frames.png`) |
| `states/04-waiting-focus--{light,dark}.png` | Waiting for a text field |
| `states/05-listening--{light,dark}.png` | Voice control listening |
| `states/06-commands-unavailable--{light,dark}.png` | Commands unavailable |
| `states/07-warning--{light,dark}.png` | Failure notice shown |
| `states/08-waiting--{light,dark}.png` | Starting / Restarting / waiting for something |
| `states/09-failed--{light,dark}.png` | Failed |
| `states/10-menu-ready--{light,dark}.png` | the menu open in the Ready state: title line, status line, separator, Settings…, Open Logs Folder, separator, Quit voicepuncher |
| `states/11-menu-recording--{light,dark}.png` | the menu open while recording (status line "Recording — release Right Option to stop") |
| `states/12-menu-failed--{light,dark}.png` | the menu open in the Failed state with the reason |
| `menu-bar-overview.png` | every icon state on one page with its name and SF Symbol under it |

## 2. Rules

- The menu bar is 24 pt high on macOS 26 (22 pt before); the icon area is 22×22 pt, the symbol 16–18 pt, vertically centred; export at 2x.
- Template icons: pure black on the light bar, pure white on the dark bar, the system applies this; draw them that way. The recording badge: a 7 pt circle in `systemRed` at the lower right of the microphone (or `record.circle` in `systemRed`, 17 pt) — draw both variants in `states/02-recording-variants.png` for the user to pick.
- SF Symbols by state: Ready `mic`; Recording `mic.fill` + red badge (or `record.circle`); Processing `waveform` (frames: `waveform`, `waveform.badge.magnifyingglass` is not it — use three `waveform` frames with different bar heights, drawn by hand in the mockup; the implementation may use a 3-frame image set); Waiting for focus `keyboard`; Listening `ear`; Commands unavailable `ear.trianglebadge.exclamationmark`; Warning `exclamationmark.triangle`; Waiting `hourglass`; Failed `exclamationmark.triangle.fill`.
- The menu: standard `NSMenu` look (system font 13 pt, 20 pt row height, separators), title line and status line disabled (grey). Keyboard shortcuts shown on the right: `⌘,` for Settings…, `⌘Q` for Quit.
- Status line texts (English, exactly these): `Ready — hold Right Option to dictate` (Hold mode) / `Ready — press Right Option to dictate` (Toggle mode); `Recording — release Right Option to stop` / `Recording — press Right Option to stop`; `Recognizing speech…`; `Refining text…`; `Waiting for a text field…`; `Voice control: listening`; `Voice control: commands unavailable`; `Starting…`; `Restarting…`; `Failed: REASON`.
- Title line: `voicepuncher 0.27.0` (the version from the crate).

## 3. Acceptance

A screenshot of the real menu bar in each state (light and dark appearance) is compared with the picture of that state; the symbol, its size and the badge must match; the menu rows and their order must match `10-menu-ready`.
