## 1. Crate test-steps and the need filter

- [x] 1.1 Red: unit test in `test-steps` — `required_env` returns a set value and panics naming the variable and the recipe when it is unset; run it, see it fail
- [x] 1.2 Green: workspace member `test-steps/` (`voicepuncher-test-steps`, exact versions, `rust-version`), `required_env`; dev-dependency of `app`, `command-spotting`, `e2e-test`; `stage_inputs` line and build-input lists
- [x] 1.3 Red: `justfile` self-test `_test-gates` — with a need's condition unmet `_gates` prints its exclusion and one `● skipped N test(s): <need> — <reason>` line; with every condition met it prints no exclusion; `_test-filter`, `_smoke-part-filter`, `_it-filter` AND it in; run it, see it fail
- [x] 1.4 Green: `_gates` recipe with the need table (D1, D2); the three filter builders and the recipes calling them print its lines
- [x] 1.5 `just test`; commit

## 2. The gated tests become needs

- [x] 2.1 Move every service, model, recordings, corpus and hang-probe test into its `needs_*` module and replace its early return with `required_env` (`commandspotting_websocket`, `transcription_openai`, `chunked_transcription`, `voice_forms`, `voice_samples`, command-spotting `server`, `stall`, `placed_commands`, `engine`, `e2e-test` `hang_probe`); `VOICEPUNCHER_HANG_PROBE` recipes still select the probe
- [x] 2.2 Delete the two `VOICEPUNCHER_TEST_WAV` tests of `recording_filesystem` (covered by the test that writes its own WAV)
- [x] 2.3 `keypressing_macos` into `needs_real_keyboard`; `test-real-keyboard` also runs it
- [x] 2.4 `just test` shows the skip lines and no `GATED:`; commit

## 3. Stage it-real

- [x] 3.1 Red: `_stages` self-test — a stage exiting 77 shows `–` skipped, is journaled `skipped`, records no pass and the run goes on; `full_check_model_stages` self-test lists `it-real`; run it, see it fail
- [x] 3.2 Green: exit 77 handling in `_stages`, `_model-stages` and the journal; `it-real` classified as a model stage, in `own_services`, in `full_check_model_stages` before `e2e-cues`; `just it-real`
- [x] 3.3 `tcc-probe` binary in `e2e-test` (Microphone, Accessibility via `MacOsPermissionProbe`); `_require-tcc`; `_it-real` runs the `needs_desktop` tests or exits 77 with the missing permission and the count
- [x] 3.4 Desktop tests into `needs_desktop` (`recording_cpal`, `focus_macos`, `menubar_macos`, `settings_macos` window tests, `main_thread` listed names); `should_draw_every_tab` draws into a temporary folder by default and checks every PNG
- [x] 3.5 `just test`; `just it-real` once; commit

## 4. Every test checks something

- [x] 4.1 Red: `architecture.rs` rule `test_checks` (D4 a–c) over the four members and the `main_thread` lists; run it, see it fail on today's offenders
- [x] 4.2 Green: give each offender a real check or delete it (`focus_macos`, `visualfeedback_macos`, `ptt_rdev`, `soundplaying_macos`, sound player unit tests, `recording_failure`, `archiving`, others the rule names)
- [x] 4.3 `just test`; commit

## 5. Paste tests on a pasteboard of their own

- [x] 5.1 Red: `textpasting_macos` test — a paste through a uniquely named pasteboard leaves the general pasteboard's change count unchanged; run it, see it fail
- [x] 5.2 Green: the paster and its `pasteboard` functions take the pasteboard by name; the tests use `pasteboardWithUniqueName`, drop `SavedUserPasteboard`, `PasteboardLock` and its test
- [x] 5.3 `just test`; commit

## 6. Shared steps

- [x] 6.1 Red: architecture rule (D4 d) — a step text defined in both `app/tests/steps/` and `e2e-test/tests/steps/` fails; run it, see it fail on the 14 texts
- [x] 6.2 Green: `SharedSteps` trait and `shared_steps!` macro in `test-steps`; both harnesses implement the trait, call the macro and delete their own definitions; "the overlay is hidden" as a Given checks in both
- [x] 6.3 `just test-one` on the slice scenarios using the shared texts; `just test`; commit

## 7. Docs and check

- [x] 7.1 `AGENTS.md`: layout lists `test-steps/`; § How to verify a change — needs and skip lines, `it-real` in `full-check` and the manual run, `test-real-keyboard` for key presses; README § Testing likewise
- [x] 7.2 No version bump: no release (proposal § Release: tests and recipes only)
- [x] 7.3 `just test`; `just check` once; commit
