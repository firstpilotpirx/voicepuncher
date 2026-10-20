# Design

## Context

The release today is `voicepuncher-vX.Y.Z-aarch64-apple-darwin.tar.gz` (binary + README) made by `_release-package` in the `justfile` with `gh release create`. The permission panes attribute an unbundled binary to the terminal that runs it. The user has no Apple Developer account. The tech matrix lists `cargo-bundle` + `create-dmg` for macOS bundles; neither is in use yet.

## Decisions

**D1 — `cargo-packager` 0.11.8 makes the bundle and the DMG.** One maintained tool covers `.app` and `.dmg` from `Cargo.toml` metadata, needs no Homebrew `create-dmg`, and is pinned exactly. Alternatives: `cargo-bundle` (app only, DMG by a second tool) and a hand-written `hdiutil` script — rejected, more moving parts for the same result.

**D2 — Self-signed code-signing certificate, no notarization.** Without a Developer ID the choice is between ad-hoc (`codesign -s -`) and a self-signed certificate. Ad-hoc signatures have a designated requirement of `cdhash`, which changes every build, so macOS treats each update as a new app and asks for Microphone, Accessibility and Input Monitoring again. A self-signed certificate gives a designated requirement that names the certificate leaf, stable across builds. The price is one Gatekeeper "Open Anyway" per version, documented. Notarization and the hardened runtime are not possible without a Developer ID and are left out.

**D3 — `LSUIElement` from the first bundle.** The app is a menu bar application; a Dock icon would be wrong once `menu-bar-presence` lands and confusing before. The binary already runs `NSApplication.run()` without a window.

**D4 — The DMG is the only release asset.** The user asked for one file; the tarball is retired at once rather than kept beside the DMG, so there is one install path to document and test.

**D5 — The bundle check is a stage of `full-check`, outside the lock.** Building the bundle and mounting the DMG takes under a minute and uses no shared service, port or GPU, so it runs before the model stages without waiting for the machine lock. `check` stays without it (it is not needed after every task group).

**D6 — Identity in `.env`, guarded by a recipe.** `VOICEPUNCHER_SIGNING_IDENTITY` names the certificate; `_require-signing-identity` fails fast with the Keychain Access steps, in the style of `_require-nextest`. The certificate exists only on the user's Mac; agents in sandboxes build with `just bundle` only when it exists there too.

**D7 — Icon and DMG background from Claude Design, kept in the repository.** The PNG sources live in `app/packaging/`; the mockups folder of this change keeps the candidates and the design spec, as the overlay change did.

## Risks / Trade-offs

- [`cargo-packager` 0.11.8 changes its DMG layout keys or signing behaviour] → the version is pinned; the recipe verifies the result (`plutil`, `codesign --verify`, `hdiutil attach`) rather than trusting the tool.
- [The user's self-signed certificate expires (Certificate Assistant defaults to 365 days)] → the README says to create it with a long validity (10 years) and what happens on expiry (a new certificate means one more round of permissions).
- [Gatekeeper rules change again in a later macOS] → the README section is the only place to update.
- [`--deep` signing misses files added later (engines)] → `bundled-engines` signs each nested binary explicitly before the app.

## Test levels

| Scenario | Level |
|---|---|
| Info.plist carries the identifier, version, `LSUIElement`, microphone usage description, icon | unit test in `app/tests/packaging_it.rs` reading `app/packaging/Info.plist` and `Cargo.toml` |
| The release is one DMG with the app and the Applications shortcut | `bundle` stage of `full-check` (`just dmg`, mount, list) |
| The signature verifies and the designated requirement names the certificate leaf | `bundle` stage of `full-check` (`codesign --verify --deep --strict`, `codesign -d -r-`) |
| `just bundle` refuses without the identity | recipe test: `VOICEPUNCHER_SIGNING_IDENTITY=missing just bundle` exits 1 with the steps (a `just` recipe `_test-bundle-guard` in the `bundle` stage) |
| Permissions survive an update | manual run by the user (two builds, reinstall) |
| Open Anyway flow on a downloaded DMG | manual run by the user |
