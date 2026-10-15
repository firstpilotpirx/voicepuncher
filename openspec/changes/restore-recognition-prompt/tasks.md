# Tasks

Start only after `limit-confirmation-fallback` is on `main` (design.md § Measure before and after).

## 1. Measure the baseline and settle the budget

- [ ] 1.1 On the branch base, record the baseline of every row of design.md § Measure before and after: `voice_samples_it`, the E2E voice control scenarios, the confirmation time of the 165 Russian windows (add the replay as an `#[ignore]`d measurement test in `e2e-test/`, run on demand), dictation WER on the 100 recordings with and without a prompt; write the numbers into design.md § Results
- [ ] 1.2 Read the prompt token count of the command prompt from whisper-server's log (one request with `max_context` large and `carry_initial_prompt`); pick B for confirmations
- [ ] 1.3 Compare design.md options 1 and 2 for dictation on WER and `just e2e-seams`; record the choice and its numbers in design.md § Decisions and § Results
- [ ] 1.4 `just check`, then commit (`test: measure recognition before the prompt takes effect`)

## 2. Reproduce the dropped prompt

- [ ] 2.1 Red: add the rule "The configured prompt steers recognition" with both scenarios and the rule "Text of one request never steers another" to `e2e-test/tests/features/speech_recognition.feature`, plus steps in `e2e-test/tests/steps/speech_recognition.rs` (a synthesized sentence with a domain term whisper misses without the prompt; a > 30 s chunk with the term after 30 s; two unrelated dictations in a row)
- [ ] 2.2 Red: add the rule "Confirmations are steered toward the command phrases" with both scenarios to `e2e-test/tests/features/voice_control.feature`, plus steps in `e2e-test/tests/steps/voice_control.rs`
- [ ] 2.3 Run the new scenarios; see the prompt scenarios fail because the prompt is dropped (the echo and "no command" scenarios may already pass: they guard against regressions)
- [ ] 2.4 Commit the failing scenarios on their own (`test: reproduce the dropped recognition prompt`, stating it is intentionally red)

## 3. Make confirmations use the prompt

- [ ] 3.1 Red: in `command-spotting/src/confirm.rs` assert the multipart body holds `carry_initial_prompt` = `true` and `max_context` = B; `just test` fails
- [ ] 3.2 Green: add both fields, with a comment on why B and why `carry_initial_prompt` (design.md § Decisions); `just test` passes
- [ ] 3.3 Run the voice control scenarios, `voice_samples_it` and the confirmation-time measurement; compare with the baseline. If a bar is missed, keep the prompt off for confirmations and reword the voice-control requirement (design.md § Measure before and after)
- [ ] 3.4 `just check`, then commit (`fix(command-spotting): make confirmations use the command prompt`)

## 4. Make dictation use the prompt

- [ ] 4.1 Red: in `app/src/adapters/secondary/transcription/openai/request.rs` assert that a non-empty prompt adds `carry_initial_prompt` = `true` and `max_context` per the chosen option, and that an empty prompt adds neither; `just test` fails
- [ ] 4.2 Green: add the fields; `just test` passes
- [ ] 4.3 Run the speech recognition scenarios, `just e2e-seams` and the WER measurement; compare with the baseline. If a bar is missed, follow design.md § Measure before and after
- [ ] 4.4 `just check`, then commit (`fix(transcription): make dictation use the configured prompt`)

## 5. Correct the descriptions

- [ ] 5.1 Update the `whisper_args` comment in `justfile`: `-mc 0` blocks text between windows of a request (requests never share text: whisper-server clears it per request); requests with a prompt set their own budget
- [ ] 5.2 Update the whisper-server start command notes in `README.md` the same way
- [ ] 5.3 Update the doc comment of `prompt` in `app/src/config/transcription.rs` to say it is used in every window of a chunk
- [ ] 5.4 Fill design.md § Results with the after numbers
- [ ] 5.5 `just check`, then commit (`docs: what -mc 0 and the prompt do`)

## 6. Release

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to the next patch version (0.13.2 if 0.13.1 is released); `just check`, then commit (`chore: release 0.13.2`)
- [ ] 6.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions do not change)
