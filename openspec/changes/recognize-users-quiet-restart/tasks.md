# Tasks

## 1. Reproduce the quiet restart (Red)

- [ ] 1.1 Remove the `KNOWN_WRONG` entry for session-2 27.23 s in `app/tests/voice_forms_it.rs` (the list stays, empty) and `UNSTEADY_RECORDINGS` with its filter in `e2e-test/tests/steps/voice_control/session/speech_after_start.rs` (design D4); `voice_forms_it` run on own ports (9280–9282) fails at session-2 27.23 s with "Let's start"; commit as `test: reproduce the user's quiet restart heard as "Let's start"` (intentionally red)

## 2. The steady clip

- [ ] 2.1 Red: unit tests in `command-spotting/src/engine/clip.rs` (design § Test levels): the first sound at the same place wherever the audio starts, the faint sound under the onset left in the lead, the lead cut to 0.25 s and padded with silence when the audio starts later, the gain only below the target, capped at 10, never below 1, the origin counting the padding; `just test` fails to compile or fails
- [ ] 2.2 Green: `steady_clip` in `command-spotting/src/engine/clip.rs` with `ONSET_SHARE` 0.4, `LEAD_SECS` 0.25, `TARGET_LEVEL` 0.1, `MAX_GAIN` 10, using the loudness helpers of `pause.rs` (design D3); `just test` passes; `kit verify after-group`, commit

## 3. The near miss

- [ ] 3.1 Red: unit tests in `command-spotting/src/matcher.rs`: `near_miss` gives "restart dictation" for "Start." and "Let's start." while dictating, none in standby, none for "let's start with the budget", "we start at nine tomorrow", "we are all here, let's start" and "Restart."; `just test` fails
- [ ] 3.2 Green: `NEAR_MISSES` ("start" → "restart dictation") next to `ALIASES` and `PhraseMatcher::near_miss` (design D3); `just test` passes; `kit verify after-group`, commit

## 4. The second hearing

- [ ] 4.1 Red: engine integration tests in `command-spotting/tests/engine_it.rs` with a scripted `Hear`: a short utterance heard "Let's start." then "Restart." while dictating gives one "restart dictation"; "Let's start." twice gives none; a long utterance ending in "let's start" asks no second hearing; `just test` fails
- [ ] 4.2 Green: `ask` takes the audio to send; a `Rehear` job context; `apply` asks the second hearing with the steady clip for a near miss of a short segment and moves the hold to it; its answer counts only as the near-missed command, placed by the second clip's origin; a debug log of lead, gain and answer (design D3); `just test` passes, and `voice_forms_it` on own ports passes at session-2 27.23 s
- [ ] 4.3 Add *Second hearing* and *Steady clip* to `docs/ubiquitous-language.md`; `kit verify after-group`, commit

## 5. E2E scenarios

- [ ] 5.1 Red: in `e2e-test/tests/features/voice_control.feature` and its steps, the Scenario Outlines of design § Test levels — the user's quiet restart after 0.3, 0.4 … 0.8 s of silence while dictating; the six command phrases and six short forms at the quiet level in their states; "start" and "let's start" on their own after a sentence, "we are all here, let's start" and "the meeting is about to start", "let's start with the budget" and "we start at nine tomorrow", at `say`'s level and at the quiet level, while dictating — with the quiet level helper in `e2e-test/src/voice_commands.rs`; the restart scenario fails against a command spotting service built without group 4 (run on own ports)
- [ ] 5.2 Green: the new scenarios and the restart rows of "The user's recorded command alone, a short pause, then speech" pass on own ports with the branch's service; `kit verify after-group`, commit

## 6. Version and checks

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to 0.14.4; `just test` passes
- [ ] 6.2 `just check` on own ports (`VOICEPUNCHER_STT_PORT=9281 VOICEPUNCHER_LLM_PORT=9280 VOICEPUNCHER_KWS_PORT=9282`) passes
- [ ] 6.3 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions do not change)
