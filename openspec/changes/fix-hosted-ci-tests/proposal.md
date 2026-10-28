## Why

Hosted CI (GitHub Actions macOS runner, `.github/workflows/ci.yml`, `just test`) has never passed. While the repository was private, every run died in about 3 seconds before any test ran; after the repository went public on 2026-10-28, run 40763356102 of firstpilotpirx/voicepuncher reached the tests and failed 6 of 1710. All 6 pass on the developer's machine, so they depend on what the runner lacks (a desktop session, a usable pasteboard, a pointer) or on timing a loaded runner cannot keep. As long as CI is red, it is no floor for pull requests opened from another machine.

Failures in that run:

1. `voicepuncher::it` `macos::textpasting_macos::should_not_overwrite_a_copy_made_after_the_paste`: `pasteboard.writeObjects(...)` returns false (`app/tests/it/macos/textpasting_macos.rs:74`).
2. `voicepuncher::it` `macos::textpasting_macos::should_leave_the_general_pasteboard_alone_when_pasting_through_another`: "the paste must put its text on the pasteboard" (`textpasting_macos.rs:91`).
3. `voicepuncher::it` `recording_cpal::synthetic::should_report_a_stream_gone_silent_for_a_second_as_a_recording_failure`: `failures.recv_timeout(Duration::from_millis(900)).is_err()` fails (`app/tests/it/recording_cpal/synthetic.rs:256`): a timing bound on a slow runner.
4. `voicepuncher::cucumber_integration` `component/dictation_pointer/the_dictation_sign_follows_the_pointer_without_getting_in_the_way`: step "Then it stands right of and below the pointer".
5. `voicepuncher::cucumber_integration` `component/local_services/an_adopted_service_that_hangs_is_replaced_by_the_app_s_own`: step "Then after three failed checks the other program keeps running and the log names it with its process id" (38 s).
6. `voicepuncher-dev-tools::cucumber_workflow` `scenarios`: step "Then it fails naming the compile error".

## What Changes

- Hosted CI runs `just test` green on the GitHub macOS runner.
- Tests that need a desktop session, the real pasteboard or the pointer move under the matching need module (`needs_desktop`), so CI leaves them out and the recipes print them as skipped; they still run where the desktop is there (`it-real`, `full-check`).
- Timing tests get bounds that hold on a loaded runner without loosening what they prove.
- The compile-error scenario of the change workflow is fixed so it holds with the runner's toolchain.

Current stage: PoC. Target stage: PoC.
Scope: the 6 tests above and whatever the runner needs to run them or leave them out; `.github/workflows/ci.yml` when a setting there is the cause.
Acceptance bar: a hosted CI run on `main` passes, and the 6 tests still pass locally in the recipes that run them.
Out of scope: any product behavior; other tests; making CI run model, service or E2E stages.

## Capabilities

### Modified Capabilities

- none: test placement and test bounds only; no requirement changes.

## Impact

- `app/tests/it/macos/textpasting_macos.rs`, `app/tests/it/recording_cpal/synthetic.rs`, the steps of `dictation_pointer` and `local_services` in `app/tests/steps/`, the `change-workflow` steps in `dev-tools/tests/`, possibly `.config/stages.toml` (`test_needs`) and `.github/workflows/ci.yml`.

## Open points

- Whether failures 4 and 5 lack a desktop or a timing margin is not known from the log; proposed: read the failing assertion first, then move it under `needs_desktop` (4) and give 5 a bound from the runner's measured time.
- Why "fails naming the compile error" breaks on the runner is not known; proposed: compare the runner's compiler message with what the step expects.

## Release

SemVer: no release — test-only change, no behavior change.
Target version: 0.41.5.
