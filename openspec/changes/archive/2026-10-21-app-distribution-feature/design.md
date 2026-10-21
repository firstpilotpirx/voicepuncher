# Design

## Decisions

### 1. E2E scenarios in the full check's bundle stage

The `app-distribution` scenarios that a machine can check are about the real bundle and disk image, which only the release build makes (`just bundle`, `just dmg`). Building them in `just test` would add a release build to every edit, and a component step that assembled its own bundle would test a copy of the recipe, not the recipe. So `e2e-test/tests/features/app_distribution.feature` is `@e2e`, tagged `@bundle`, and runs in the `bundle` stage of `full-check`, the one stage that already builds both, outside the machine lock: `_bundle` builds the disk image, keeps its shell checks of the bundle (now the recipe `_check-bundle <app> <dmg>`), then runs `cargo nextest run --profile bundle` on the E2E harness with `--tags "@bundle and not @wip"`. The shell guard tests of `_test-bundle-guard` move into scenarios and the recipe goes. `just test` and `check` stay without them, as the spec asks.

*Alternatives:* a component feature over a test-built archive (the proposal's first idea: a release build per `just test`, or a copy of the recipe); a recipe of its own (one more command nobody runs).

### 2. Builds a scenario makes go to a directory of its own

Scenarios that run `just bundle` set the justfile variable `dist_dir` to the scenario's temporary directory (`just --set dist_dir <dir> bundle`), so an ad-hoc build never replaces the signed `dist/voicepuncher.app` the stage just checked, and scenarios can run at once. Scenarios that read the stage's own build take `dist/` of the checkout. The release guard runs `_require-release-identity`, the first dependency of `release`, never `release` itself: a broken guard must not publish.

### 3. Two builds, one identity, behind a gate

"Two builds share one identity" needs the signing certificate. A new harness gate `@signing_identity` opens when `VOICEPUNCHER_SIGNING_IDENTITY` is set and not empty, closed with a `GATED:` line otherwise, like `@voice_samples`. The second build is a copy of the stage's bundle with a different build version in its information file, signed again with the same identity: a different signed build without a second release build; both designated requirements must name the certificate and be equal.

### 4. What stays manual

Scenarios that need a person, the Finder, the permission panes or GitHub stay `@wip` with the reason in a comment: a release is published (publishes a GitHub Release), permissions listed under the application's name, no Dock icon, permissions survive an update, a user installs, a maintainer prepares a release machine. "The disk image opens as an installer" checks what a machine can: the volume name, the bundle, the Applications shortcut, the background picture and the window layout file; where the Finder draws the icons stays with the release checklist. "The full check fails on a broken bundle" runs the stage's bundle check on a bundle whose information file lacks the microphone description; that `_stages` stops before the model stages after a failed stage is its own tested behavior.

## Test levels

| Scenario | Level |
|---|---|
| A release is published | manual (`@wip`) |
| The disk image opens as an installer | E2E `.feature` (volume, contents); window drawing manual |
| The bundle's information matches the crate | E2E `.feature` |
| Permissions are listed under the application's name | manual (`@wip`) |
| No Dock icon | manual (`@wip`) |
| Two builds share one identity | E2E `.feature`, `@signing_identity` |
| Permissions survive an update | manual (`@wip`) |
| The signature verifies strictly | E2E `.feature` |
| The release without a signing identity | E2E `.feature` |
| The bundle build with an identity the keychain lacks | E2E `.feature` |
| The bundle build without a signing identity | E2E `.feature` |
| A user installs a downloaded release | manual (`@wip`) |
| A maintainer prepares a release machine | manual (`@wip`) |
| The full check verifies the bundle | E2E `.feature` (the full check's stage list) |
| The full check fails on a broken bundle | E2E `.feature` |
| The quick check stays without the bundle | E2E `.feature` (the quick check's stage list) |
| The `@signing_identity` gate | unit (`scenario_filter`) |
