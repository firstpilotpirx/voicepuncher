# Tasks

## 1. Packaging sources: Info.plist, icon, DMG background

- [ ] 1.1 Red: `app/tests/packaging_it.rs` — `app/packaging/Info.plist` lints with `plutil`, carries the bundle identifier, name, display name, `LSUIElement`, `LSMinimumSystemVersion`, `NSMicrophoneUsageDescription`, `LSApplicationCategoryType`, `CFBundleIconFile`, executable `voicepuncher`, and `app/packaging/icons/voicepuncher.iconset/` holds the ten standard sizes; run it and see it fail
- [ ] 1.2 Make three icon candidates per `mockups/design-spec.md` into `mockups/icons/candidate-{1,2,3}.png` with their 16/32/128 px renders and `overview.png`; pick candidate 1 as the working choice (the user confirms at the manual run) into `app/packaging/icons/icon-1024.png` and the iconset via `sips -z`
- [ ] 1.3 Make `mockups/dmg-background.png` (1200×800) per the design spec and copy it to `app/packaging/dmg-background.png`
- [ ] 1.4 Green: write `app/packaging/Info.plist`; `dist/` in `.gitignore`; `app/packaging/**` in the `permissions` area of `.claude/kit.yaml`; 1.1 passes
- [ ] 1.5 `just test` and commit

## 2. Recipes: bundle, signing, DMG

- [ ] 2.1 Red: `_test-bundle-guard` in the `justfile` — `VOICEPUNCHER_SIGNING_IDENTITY=missing just bundle` exits 1 naming the identity and the Keychain Access steps, `VOICEPUNCHER_SIGNING_IDENTITY= just bundle` succeeds and prints the ad-hoc notice, the release guard exits 1 without the variable; run it and see it fail
- [ ] 2.2 Green: `_require-create-dmg`, `_require-signing-identity`, `_sign` (named identity or ad-hoc with the notice, `--timestamp=none`, then `codesign --verify --deep --strict --verbose=2` and `codesign -d -r-`), `bundle` (release build, `Contents/MacOS`, `iconutil -c icns`, Info.plist copied with `plutil -replace` for both version keys), `dmg` (`create-dmg` with the design-spec layout into `dist/voicepuncher-vX.Y.Z.dmg`, volume `voicepuncher X.Y.Z`, `.sha256`); 2.1 passes and `plutil -p dist/voicepuncher.app/Contents/Info.plist` shows the crate's version
- [ ] 2.3 Red: `_bundle` stage in the `justfile` — `just dmg`, `plutil -lint`, every key of 1.1 present with its value, `codesign --verify --deep --strict`, `hdiutil attach -nobrowse -readonly`, the app and the `Applications` link present, detach; `full-check` lists it before `build-e2e`, `check` does not; run `just _stages bundle` with the microphone description removed and see it fail naming the key
- [ ] 2.4 Green: the `_bundle` stage passes on the restored Info.plist; `just _stages bundle` passes; `just check` lists no bundle stage (its recipe line unchanged)
- [ ] 2.5 Add Bundle, Signing identity and Disk image to `docs/ubiquitous-language.md`; `app/packaging/` and `dist/` to `docs/project-map.md`
- [ ] 2.6 `just test` and commit

## 3. Release and documentation

- [ ] 3.1 `_release-package` builds `just dmg` after `cargo clean` and the tests; `release` requires `VOICEPUNCHER_SIGNING_IDENTITY` through `_require-signing-identity`; `_release` publishes `dist/voicepuncher-vX.Y.Z.dmg` and its `.sha256` and nothing else; verify with `just --dry-run _release-package` and by reading `_release`
- [ ] 3.2 README § Install: download the DMG, drag, Open Anyway in Privacy & Security once per version (macOS 15+), the permissions appear under the app's name, quitting in this version via Activity Monitor or `pkill voicepuncher`; the build-from-source steps move under a "From source" sub-heading
- [ ] 3.3 README § Release checklist: create the certificate in Keychain Access (name, Self Signed Root, Code Signing, 10 years, Always Trust for Code Signing), `.env` line, the release now publishes the DMG, the TCC identity check (grant, rebuild, reinstall, no prompt) and the Gatekeeper check on a downloaded DMG
- [ ] 3.4 `docs/architecture/overview.md` § Technology Choices: packaging row (macOS tools + create-dmg) and signing row (self-signed certificate, no Developer ID)
- [ ] 3.5 `just test` and commit

## 4. Tech matrix (agent-kit)

- [ ] 4.1 In `/Users/me/Workspaces/my/agent-kit`, branch `docs/macos-desktop-packaging`: `claude-code/rules-custom/tech-matrix-rust.md` — replace the "macOS .app / PKG" row and add the "macOS Desktop" section (packaging, installer DMG, signing without a Developer ID, `notarytool` only with one); PR opened and merged, the kit installed (`kit install` or the kit's install recipe) so `~/.claude/rules-custom/tech-matrix-rust.md` shows the rows
- [ ] 4.2 In this repository, no file changes; record the agent-kit PR link in `design.md` § Context and commit

## 5. Release

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to the next minor version above `main` (0.26.0 at the time of writing; `kit version-check app-bundle-and-dmg`)
- [ ] 5.2 `just check` once and commit
- [ ] 5.3 Manual run by the user: create the certificate per README, `just dmg`, open the DMG (layout, background), drag to Applications, first launch → Open Anyway, grant the three permissions to voicepuncher (listed by name with the icon), dictate; change one line, `just dmg` again, reinstall, launch → no prompt, dictation works; confirm the icon candidate
