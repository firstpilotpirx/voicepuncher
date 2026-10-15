# Tasks

## 1. Reproduce the bug

- [x] 1.1 Red: move `without_lead_pause` and `without_trailing_silence` to `e2e-test/src/voice_commands.rs`, add `from_first_sound`, and make "takes a short breath" pause 0.35 s from sound to sound (D3). Add E2E `.feature` scenarios to `e2e-test/tests/features/voice_control.feature`, rule "An utterance ends after 0.3 s of silence": "Restart", 0.4 s, a sentence opening with a hard consonant ("Coffee…"), in real time; "enter", 0.3 s, a sentence, in standby; the user's recorded "restart", "start", "stop" and "enter" alone, each followed by 0.3, 0.45 and 0.6 s of silence and a sentence (`@voice_samples`; the "restart" at 27.23 s in `session-2` is left out, as whisper hears it as "Let's start" whatever the pause: backlog change `recognize-users-quiet-restart`). Add engine integration tests to `command-spotting/tests/engine_it.rs`: a `say` command, 0.32 s of silence and a sentence give two segments; the user's quiet "restart" with 0.4 s of silence and speech gives two segments; no clip that confirms the user's quiet "restart" followed by 0.6 s and a sentence holds the sentence's first sound (the detector holds that "restart" about 0.55 s past its end). Run them and see them fail; commit alone as `test: reproduce commands lost after a short pause` (intentionally red)

- [x] 1.2 After the review: "Stop after a short breath" runs in the hands-free session and checks the paste; the user's recorded forms are cut after their last sound (`until_last_sound`) before the pause; `placed_commands_it`'s breath is cut from sound to sound; `commandspotting_websocket_it`'s test is renamed to what it checks; `just check` and commit

## 2. A quiet run ends the utterance

- [x] 2.1 Red: unit tests in `command-spotting/src/engine/pause.rs`: frames at most 3 % of the segment's loudest frame, or near the room's noise floor, are quiet; a quiet run of 0.3 s ends the segment where it began; a louder frame restarts the run; nothing ends before any speech. See them fail
- [x] 2.2 Green: `PauseWatch`; the session flushes and resets the detector when it reports a pause, and ends the segment at the quiet run's start (D1). `just test`, the engine split tests of 1.1 green
- [x] 2.3 Red: unit tests in `engine/pause.rs` (`first_sound`) that the confirmation clip ends at the first sound after the segment end, and at the full post-roll without one. See it fail
- [x] 2.4a Red: `outbox.rs` unit test that a phrase of the last segment, spotted while the segment is held, goes out before its end (D2b; found by `commandspotting_websocket_it` once D1 closed segments sooner). See it fail; Green: `Outbox::send_in_last_segment`, used for the spotter's late phrases; `commandspotting_websocket_it` accepts the spotter's phrase shortly after the speech's end, as the app's recognizer does
- [x] 2.4b Red: `outbox.rs` unit test that the spotter's phrases of other commands are dropped from a segment whose confirmation heard a command (D2c; `voice_forms_it`, session-4 "restart dictation please"). See it fail; Green: `Outbox::keep_only_phrase`, called when a segment's answer is a phrase
- [x] 2.4c Red: engine integration test (`engine_it.rs`) that a command heard only in the last seconds of a 2.7 s utterance is reported (D2d; `voice_forms_it`, session-4 "we meet on Friday at ten enter"), and `outbox.rs` unit test for a segment whose wait moves to another confirmation. See them fail; Green: `JobKind::Tail`, `Outbox::transfer`
- [x] 2.4 Green: `close_segment` stops the post-roll at the next sound (D2). `just test`, the clip test of 1.1 green; `just check` and commit

## 3. Docs and release

- [x] 3.1 README voice control section: a command said on its own needs a pause of only 0.3 s before the next speech, also in a quiet voice. Glossary (`docs/ubiquitous-language.md`): check that *utterance* and *pause* still hold; add none. Commit
- [x] 3.2 Bump `version` in `app/Cargo.toml` to 0.14.1 (the next patch after `main`'s 0.14.0); `just check` and commit
- [x] 3.3 Manual checks: none (the change is inside the command spotting service; no hotkey, recording, overlay, paste or permission change)
