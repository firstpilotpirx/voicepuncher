## Why

Hosted CI (GitHub Actions macOS runner, `.github/workflows/ci.yml`: `just fmt`, `just lint`, `just test`) has never passed. Run 40763356102 reached the tests and failed 6 of 1710; every later run died earlier: `just lint` refuses Homebrew's trufflehog 3.99.7 against the `justfile` pin 3.99.6, and `just test` cannot link `command-spotting` because the Rust cache keeps the build script's output but drops the sherpa-onnx library it downloaded. All 6 tests pass on the developer's machine; the log shows each runner-only cause:

1. and 2. `macos::textpasting_macos`: the runner's session has no usable pasteboard.
3. `recording_cpal::synthetic` (stream gone silent): the audio is fed at real time, and a loaded runner stretches the feed past the watchdog's silence limit.
4. `component/dictation_pointer` (the sign follows the pointer): a real window beside the real pointer.
5. `component/local_services` (a hung adopted service): the fake hung program never accepts connections, so once its listen backlog is full macOS resets the probe's connects and the app sees the port closed.
6. `change-workflow` "fails naming the compile error": the runner's `CARGO_TERM_COLOR=always` puts color codes into the message.

A red CI is no floor for pull requests opened from another machine.

## What Changes

- The workflow installs shellcheck, gitleaks and trufflehog from their GitHub releases at the `justfile` pins (trufflehog bumped to 3.99.8) into the cached tools folder, and drops the sherpa-onnx build output when its library is missing.
- The paste tests (1, 2) move under `needs_desktop`; a `@needs_desktop` tag on a `Rule:` (4) puts the need into its test name, so `just test` leaves it out and `it-real` runs it.
- The watchdog tests (3, and its sibling on a device error) push their audio in one call.
- The fake hung program (5) accepts every connection and leaves it unanswered.
- The compile-error scenario (6) runs its static check with colors off.

Current stage: PoC. Target stage: PoC.
Scope: the 6 tests, the two runner-only workflow failures, the recipes that run the desktop tests.
Acceptance bar: the pull request's hosted CI run passes; the moved tests still pass locally in `it-real` and `test-real-keyboard`.
Out of scope: product behavior; other tests; model, service or E2E stages in CI.

## Capabilities

### Modified Capabilities

- `change-workflow`: the desktop tests include the Gherkin rules tagged `@needs_desktop`.

## Impact

- `.github/workflows/ci.yml`, `justfile`, `AGENTS.md`, `README.md`; `app/tests/it/macos/textpasting_macos.rs`, `app/tests/it/recording_cpal/synthetic.rs`, `app/tests/features/component/dictation_pointer.feature`, `app/tests/steps/visual_feedback/pointer.rs`; `e2e-test/src/harness_args.rs`, `dev-tools/src/stages/it_real.rs`, `test-steps/src/loopback/silent_listener.rs`, `dev-tools/tests/steps/test_loop/static_check.rs`.

## Open points

- Whether the other lint analyzers pass on the runner once the tools are found: the pull request's CI run shows it; a runner-only failure there is fixed here.

## Release

SemVer: no release — tests and tooling only. Target version: unchanged.
