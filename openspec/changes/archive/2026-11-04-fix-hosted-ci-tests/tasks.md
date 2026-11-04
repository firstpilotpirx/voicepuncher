## 1. The workflow and the tool pins

- [x] 1.1 `.github/workflows/ci.yml`: shellcheck, gitleaks and trufflehog from their GitHub releases at the `justfile` pins into the cached tools folder; the sherpa-onnx build output removed after the cache restore when its prebuilt folder is missing; `trufflehog_version` 3.99.8 in the `justfile`
- [x] 1.2 `just test`, commit

## 2. The desktop tests by name

- [x] 2.1 Red: unit test that a rule tagged `@needs_desktop` is named with the need; `change_workflow.feature` scenario "A business rule marked for the desktop is a desktop test" with its steps
- [x] 2.2 Green: the tag in the rule's test name (`e2e-test/src/harness_args.rs`); `it-real` runs the slice/component harness with the filter `test(/needs_desktop::/) & not test(/needs_real_keyboard::/)` (`dev-tools/src/stages/it_real.rs`, its tests and the workflow steps); `_test-real-keyboard` sets the desktop variable
- [x] 2.3 The paste tests under `needs_desktop` (`app/tests/it/macos/textpasting_macos.rs`); the pointer rule tagged `@needs_desktop`, its step requiring the desktop variable; `AGENTS.md`, `README.md` and the `it-real` comment of the `justfile` name the tagged rules
- [x] 2.4 `just test`, commit

## 3. The timing test, the fake hung program and the compile-error scenario

- [x] 3.1 The watchdog tests of `app/tests/it/recording_cpal/synthetic.rs` push their audio in one call; the fake hung program of `test-steps/src/loopback/silent_listener.rs` accepts every connection and keeps it open; the static check of `dev-tools/tests/steps/test_loop/static_check.rs` runs with colors off
- [x] 3.2 `just test`, commit
- [x] 3.3 No version bump: tests and tooling only, no behavior change (proposal § Release)
