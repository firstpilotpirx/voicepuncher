# App icon and DMG background — design spec

What to make with Claude Design before the proposal gate (stage 2), and what the user approves at stage 3 together with the proposal.

## How to make the pictures

Use Claude Design: call the `Artifact` tool with `action: "quickstart"` and `intent: "design"`, follow the Design type it names, and draw the pictures below. Export PNGs into this folder. If Claude Design is not available in the session, draw the icon as SVG (`mockups/icons/*.svg`), render it with the built-in browser at the sizes below and screenshot, as `openspec/changes/archive/2026-10-19-redesign-overlay-liquid-glass/mockups/` did with HTML.

## 1. App icon — three candidates

- Format: macOS 26 app icon. A rounded square filling the 1024×1024 canvas (corner radius ≈ 22.4 % of the side, the system mask is applied by macOS; draw the full square), a simple layered composition, a glossy but restrained look, no text.
- Themes to try (one picture each, `mockups/icons/candidate-{1,2,3}.png`, 1024 px):
  1. A microphone capsule with a short "punch" impact mark (two or three motion lines) on a deep blue-to-violet gradient.
  2. A sound wave turning into a text cursor (|) on a dark background with a warm accent.
  3. A keyboard key cap with a microphone glyph on it, in the app's accent colour.
- Each candidate also rendered at 16, 32 and 128 px (`candidate-N-{16,32,128}.png`) to prove legibility: at 16 px the silhouette must still read as a microphone.
- Light, dark and tinted appearance: show the chosen icon over the macOS light and dark menu bar background in one overview picture (`mockups/icons/overview.png`).
- The chosen candidate goes to `app/packaging/icons/icon-1024.png`; derive 512, 256, 128, 32, 16 (and the 2x ones) into `app/packaging/icons/voicepuncher.iconset/` with `sips -z`; `iconutil` builds the `.icns`.

## 2. DMG background

- Canvas 600×400 pt, exported at 2x (1200×800 px) as `mockups/dmg-background.png`, copied to `app/packaging/dmg-background.png` when approved.
- Content: left area for the app icon (centred at 150, 200 pt), right area for the Applications folder (centred at 450, 200 pt), a curved arrow between them, the text "Drag voicepuncher to Applications" at the bottom in SF Pro 15 pt, a light neutral background that reads in light and dark Finder appearance (Finder does not switch DMG backgrounds), the app version is not drawn (it changes per release).
- `create-dmg` places the icons at the positions above (`--icon voicepuncher.app 150 200 --app-drop-link 450 200` in the `dmg` recipe of the `justfile`); keep the window size 600×400.

## 3. Acceptance

The implementation is done when: the icon in the Dock-less app shows in System Settings → Privacy & Security panes and in the DMG window as the approved candidate; the DMG window opens with the background, the app at the left position and Applications at the right, at 100 % zoom on the user's display.
