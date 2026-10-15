# Tasks

## 1. Reproduce the stall

- [ ] 1.1 Red: add the rule "Speech that is no command is confirmed quickly" with both scenarios to `e2e-test/tests/features/local_services.feature` (`@resilience`), plus their steps in `e2e-test/tests/steps/local_services.rs`: stream the synthesized Russian phrases and then "press enter" to a command spotting service on ports of its own, read the confirmation times from its log, assert each is under 2 s and that "press enter" is reported
- [ ] 1.2 Run `just e2e-resilience` and see the first scenario fail on a confirmation that takes > 2 s; if no phrase reproduces it, switch to another measured phrase (design.md § Test the request at unit level, the behavior at E2E level)
- [ ] 1.3 Commit the failing scenario on its own (`test: reproduce slow confirmation of Russian speech`, stating it is intentionally red)

## 2. Limit the retries of a confirmation

- [ ] 2.1 Red: in `command-spotting/src/confirm.rs` assert that the multipart body holds `temperature_inc` = `0.55`; run `just test` and see it fail
- [ ] 2.2 Green: add the `temperature_inc` field with `0.55` to the confirmation request, with a comment why 0.55 gives exactly two attempts; `just test` passes
- [ ] 2.3 Run `just e2e-resilience`; both new scenarios pass
- [ ] 2.4 `just check`, then commit (`fix(command-spotting): limit the retries of a confirmation`)

## 3. Correct the descriptions of `-nf`

- [ ] 3.1 Update the `whisper_args` comment in `justfile`: whisper-server 1.9.5 ignores `-nf`; dictation keeps the server's temperature fallback; confirmations set their own retry step
- [ ] 3.2 Update the whisper-server start command notes in `README.md` the same way
- [ ] 3.3 Update the hint for the outdated `temperature_inc` setting in `app/src/config/transcription.rs` so it no longer says `-nf` disables the fallback; adjust its unit test if it asserts the text
- [ ] 3.4 `just check`, then commit (`docs: whisper-server 1.9.5 ignores -nf`)

## 4. Release

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.13.1; `just check`, then commit (`chore: release 0.13.1`)
- [ ] 4.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions do not change)
