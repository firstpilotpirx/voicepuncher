# Design

## Context

The release today is `voicepuncher-vX.Y.Z-aarch64-apple-darwin.tar.gz` (binary + README) made by `_release-package` in the `justfile` with `gh release create`. The permission panes attribute an unbundled binary to the terminal that runs it. The user has no Apple Developer account. The tech matrix lists `cargo-bundle` + `create-dmg` for macOS bundles; neither is in use yet.

## Decisions

**D1 — Plain `just` recipes on macOS tools make the bundle; `create-dmg` lays out the DMG.** A bundle is three files in a fixed tree: the recipe copies the release binary, fills the Info.plist template with `plutil` and builds the `.icns` with `iconutil`; `codesign` signs, `hdiutil` verifies. The DMG window (background, icon positions, Applications link) needs Finder scripting, which `create-dmg` 1.3.0 (Homebrew, pinned like the other developer tools) already does. `cargo-packager` 0.11.8 was read at explore and rejected: its signing always adds `--options runtime` (the hardened runtime, which without entitlements blocks the microphone and the key listener) and `--timestamp`; its DMG is built from a bundle it re-creates unsigned in the same run, leaving no place for our own signature; and it downloads `create-dmg` from GitHub at build time. `cargo-bundle` was rejected for the same reason: it adds a crate for what `plutil` and `iconutil` do.

**D2 — Self-signed code-signing certificate, no notarization.** Without a Developer ID the choice is between ad-hoc (`codesign -s -`) and a self-signed certificate. Ad-hoc signatures have a designated requirement of `cdhash`, which changes every build, so macOS treats each update as a new app and asks for Microphone, Accessibility and Input Monitoring again. A self-signed certificate gives a designated requirement that names the certificate leaf, stable across builds. The price is one Gatekeeper "Open Anyway" per version, documented. Notarization and the hardened runtime are not possible without a Developer ID and are left out.

**D3 — `LSUIElement` from the first bundle.** The app is a menu bar application; a Dock icon would be wrong once `menu-bar-presence` lands and confusing before. The binary already runs `NSApplication.run()` without a window.

**D4 — The DMG is the only release asset.** The user asked for one file; the tarball is retired at once rather than kept beside the DMG, so there is one install path to document and test.

**D5 — The bundle check is a stage of `full-check`, outside the lock.** Building the bundle and mounting the DMG takes under a minute and uses no shared service, port or GPU, so it runs before the model stages without waiting for the machine lock. `check` stays without it (it is not needed after every task group).

**D6 — Identity in `.env`, guarded by a recipe; ad-hoc without it.** `VOICEPUNCHER_SIGNING_IDENTITY` names the certificate; `_require-signing-identity` fails fast with the Keychain Access steps, in the style of `_require-nextest`. The certificate exists only on the user's Mac, and `full-check` must pass in every worktree and agent sandbox: with the variable unset, `just bundle` signs ad-hoc (`codesign -s -`) and prints that the bundle has no stable identity; `just release` always requires the variable. A set variable that names no identity in the keychain is an error, never a silent fallback.

**D7 — Icon and DMG background from Claude Design, kept in the repository.** The PNG sources live in `app/packaging/`; the mockups folder of this change keeps the candidates and the design spec, as the overlay change did.

## Risks / Trade-offs

- [`create-dmg` changes its options or Finder scripting breaks on a later macOS] → the recipe verifies the result (`hdiutil attach`, the app and the link present) rather than trusting the tool; the layout is cosmetic, the install still works without it.
- [The user's self-signed certificate expires (Certificate Assistant defaults to 365 days)] → the README says to create it with a long validity (10 years) and what happens on expiry (a new certificate means one more round of permissions).
- [Gatekeeper rules change again in a later macOS] → the README section is the only place to update.
- [Signing misses files added later (engines)] → `bundled-engines` signs each nested binary explicitly before the app; `codesign --verify --deep --strict` in the `bundle` stage catches an unsigned nested file.

## Test levels

| Scenario | Level |
|---|---|
| A release is published | recipe reading: `_release` publishes the two files of `dist/` and nothing else (`just --dry-run`), confirmed by the release of this change |
| The disk image opens as an installer | manual run by the user (the `bundle` stage of `full-check` mounts it and checks the app and the Applications link) |
| The bundle's information matches the crate | adapter integration test `app/tests/packaging_it.rs` (the template's keys) and the `bundle` stage of `full-check` (the version keys of the built bundle) |
| Permissions are listed under the application's name | manual run by the user |
| No Dock icon | manual run by the user (`LSUIElement` checked by `app/tests/packaging_it.rs`) |
| Two builds share one identity | manual run by the user (two builds, `codesign -d -r-`) |
| Permissions survive an update | manual run by the user (two builds, reinstall) |
| The signature verifies strictly | `bundle` stage of `full-check` (`codesign --verify --deep --strict`) |
| The release without a signing identity | recipe test `_test-bundle-guard` in the `bundle` stage |
| The bundle build with an identity the keychain lacks | recipe test `_test-bundle-guard` in the `bundle` stage |
| The bundle build without a signing identity | recipe test `_test-bundle-guard` in the `bundle` stage |
| A user installs a downloaded release | manual run by the user on the DMG downloaded from the release |
| A maintainer prepares a release machine | manual run by the user (creating the certificate) |
| The full check verifies the bundle | the `bundle` stage of `full-check` itself, in the stage table of the full check of this change |
| The full check fails on a broken bundle | recipe test: `just _stages bundle` with the microphone description removed (task 2.3) |
| The quick check stays without the bundle | reading the `check` recipe line; `just check` of this change lists no bundle stage |
