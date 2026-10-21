# Settings window — design spec

What to make with Claude Design before the proposal gate (stage 2); the user approves at stage 3 with the proposal; the implementation is done when a screenshot of each tab in the manual run matches this document (no pictures were made: design D13).

## How to make the pictures

Call the `Artifact` tool with `action: "quickstart"` and `intent: "design"`, follow the Design type it names, and draw the window as a macOS 26 settings window at 2x (SF Pro, system controls, light and dark). Export PNGs into `mockups/states/`. (None were made: see design D13; this document is the reference.) Without Claude Design: one static HTML page per tab in `mockups/settings.html?tab=…&bg=…`, screenshots with the built-in browser.

## 1. The window

- Title: `voicepuncher Settings`. Width 560 pt; height follows the tab (420–620 pt). Toolbar-style tabs across the top (icon above label, the selected one highlighted), as System Settings panes on macOS 26: General `gearshape`, Dictation `waveform`, Refinement `sparkles`, Sounds `speaker.wave.2`, Paste `doc.on.clipboard`, Archive `archivebox`, Logging `doc.text` (Services is added by `model-download`: leave room).
- Content: a form grid, labels right-aligned in a 160 pt column, controls left-aligned, 20 pt margins, 10 pt row spacing, section titles in bold 13 pt where a tab has groups, `Show advanced` as a disclosure triangle revealing extra rows.
- Bottom bar: a status label at the left (secondary colour; `Saved` / `Applying after the current dictation…` / `Applied` / the error in red), `Revert` and `Save` (default) at the right.
- Error state: the offending control with a 1 pt red outline and the message in the status label.

## 2. Tabs and rows

General: Hotkey (pop-up), Recording mode (two radios), Minimum press (number + stepper, `ms`), Microphone (pop-up), Longest recording (number, `s`), Voice control (switch); advanced: Voice control service (text).
Dictation: Language (pop-up), Languages to expect (text, help text `comma-separated codes, e.g. en, ru`), Prompt (text view 90 pt), Temperature (slider with value), Timeout (`s`); advanced: Chunk threshold, Chunk overlap, VAD lookback (`s`), Service address (text).
Refinement: Refine text with the language model (switch; with the note `Set to Off by the command line for this run` under it when `--no-llm` is set, still editable: Save writes the file value), Model (text), System prompt (text view 120 pt), Examples (table 2 columns `Transcript`, `Normalized`, +/− under it, 4 visible rows), Temperature, Max tokens, Timeout; advanced: Service address, API key.
Sounds: nine rows (Start, Stop, Cancel, Listening on, Listening off, Enter, Clear, Paste, Restart): pop-up of 14 names + `None`, a ▶ button after it.
Paste: Clipboard restore delay (`ms`), Wait for a text field (`s`).
Archive: Keep every dictation (switch), Folder (path control + `Choose…`), Size limit (`MB`), `Show in Finder` button.
Logging: Level (pop-up), `Open Logs Folder` button.

## 3. Pictures to export (not made: design D13)

Planned and not made; this document is the reference. `states/01-general--{light,dark}.png` … `states/07-logging--{light,dark}.png` (one per tab, filled with the default values), `states/08-general-error--light.png` (Minimum press = 5 refused), `states/09-applying--light.png` (status `Applying after the current dictation…`), `states/10-unsaved-alert--light.png` (the Save / Don't Save / Cancel sheet), `states/11-refinement-off-by-flag--light.png`, `settings-overview.png` (every tab on one page).

## 4. Strings

Labels and help texts as in § 2; status texts `Saved`, `Applying after the current dictation…`, `Applied`; error form `Minimum press must be between 50 and 1000 ms`, `Service address must be on this Mac (127.0.0.1 or localhost)`; alert `You have unsaved changes.` with `Save`, `Don't Save`, `Cancel`.

## 5. Acceptance

Screenshots of the real window on each tab (light and dark) match this document: same rows in the same order, same control kinds, aligned labels, the bottom bar.
