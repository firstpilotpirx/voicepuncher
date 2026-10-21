# Permissions window — design spec

What to make with Claude Design before the proposal gate (stage 2); the user approves the pictures at stage 3 with the proposal; the implementation is done when a screenshot of each state in the manual run matches its picture.

## How to make the pictures

Call the `Artifact` tool with `action: "quickstart"` and `intent: "design"`, follow the Design type it names, and draw the window below as a macOS 26 window (system font SF Pro, system controls, 2x). Export PNGs into `mockups/states/`. Without Claude Design: one static HTML page `mockups/permissions.html` with `?state=` and `?bg=light|dark`, screenshots with the built-in browser (as the overlay change did).

## 1. The window

- Title bar: `Permissions`. Size 460×390 pt, not resizable. Standard window chrome, light and dark appearance.
- Heading (bold 15 pt): `voicepuncher needs three permissions`. Subheading (secondary label, 13 pt): `Grant each one below. voicepuncher checks again by itself; no restart is needed.`
- Three rows, 20 pt margins, 12 pt between rows, each row: a 28 pt SF Symbol in a 36×36 pt rounded tile (symbols `mic`, `accessibility`, `keyboard`; tile tint system blue), then two lines of text (name in 13 pt medium: `Microphone`, `Accessibility`, `Input Monitoring`; why in 12 pt secondary: `to hear your dictation`, `to paste the text into the app you are typing in`, `to notice the hotkey in any app`), then at the right a status badge and a button stacked or side by side (stacked: badge above button; the button 140 pt wide, so `Open System Settings` fits on one line; the why line wraps to a second line when it is long).
- Status badges: `Granted` (green dot + text), `Not granted` (red dot), `Not asked yet` (grey dot), `Checking…` (small spinner).
- Buttons per status: `Request…` (Not asked yet), `Open System Settings` (Not granted), none when Granted (badge only).
- Developer case (unbundled): a line under the heading in the warning colour: `voicepuncher runs from iTerm2 — grant the permissions to iTerm2.`
- Footer: left `Quit`, right `Continue` (default button; enabled only when all three are granted).

## 2. Pictures to export

| File | State |
|---|---|
| `states/01-none--{light,dark}.png` | all three Not asked yet |
| `states/02-mixed--{light,dark}.png` | Microphone granted, Accessibility not granted, Input Monitoring not asked yet |
| `states/03-checking--{light,dark}.png` | one row Checking… after Request… |
| `states/04-all-granted--{light,dark}.png` | all granted, Continue enabled |
| `states/05-developer--{light,dark}.png` | the unbundled case with the warning line naming iTerm2 |
| `states/06-revoked--{light,dark}.png` | the window reopened with the revoked row red and a line `Accessibility was revoked — dictation stopped` |
| `permissions-overview.png` | all states on one page with notes |

## 3. Strings (English, exactly)

Heading, subheading, row names and reasons as above; badges `Granted`, `Not granted`, `Not asked yet`, `Checking…`; buttons `Request…`, `Open System Settings`, `Quit`, `Continue`; developer line `voicepuncher runs from APP — grant the permissions to APP.`; revoked line `PERMISSION was revoked — dictation stopped`.

## 4. Acceptance

Screenshots of the real window in states 01, 02, 04 and 05 (light and dark) are compared with the pictures: same rows, same badges, same buttons, same proportions.
