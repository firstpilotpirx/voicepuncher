# Tasks

## 1. Measure

- [x] 1.1 Red: slice scenarios "A voice start logs its timing" (sound and overlay stages) in `voice_control.feature`; run with `just test-one`, see them fail
- [x] 1.2 Green: `VoiceStartTiming` gains sound and overlay; `VisualIndicator::show_recording` takes an `on_drawn` callback; the line is logged from it
- [x] 1.3 Service logs which path closed each segment (pause watch or voice detector; a half pause and a spotted check close none), unit test in `command-spotting/`
- [x] 1.4 Record the before numbers: replay the user's voice samples and `just e2e-cues`, write them into `design.md`
- [x] 1.5 `just test`; commit

## 2. App: overlay and sound with the cue

- [x] 2.1 Red: slice scenario "The overlay shows with the start cue"; see it fail
- [x] 2.2 Green: sound and overlay before `recorder.start()` in `pipeline/recording.rs`
- [x] 2.3 Red: adapter test "The recording sign is drawn at once" and "A screen added later gets the overlay" (`needs_desktop`)
- [x] 2.4 Green: `show_state` keeps the windows; the screen watcher rebuilds them
- [x] 2.5 `MacosSoundPlayer` preloads `NSSound`s and plays them in place; adapter test that every cue loads
- [x] 2.6 `just test`; commit

## 3. App: one microphone

- [x] 3.1 Red: integration test: while voice control listens, a dictation opens no second input device and records the kept microphone's audio
- [x] 3.2 Green: a fan-out of the kept microphone as the recorder's source (`CpalAudioRecorder::with_source`)
- [x] 3.3 `just test`; commit

## 4. Service: event-driven loop and pause

- [x] 4.1 Red: unit test: an answer is collected without waiting for the 20 ms poll
- [x] 4.2 Green: the processing thread selects on audio and answers; read waits 2 ms
- [x] 4.3 Red: E2E scenario "A command in a noisy room ends at its pause"; see it fail
- [x] 4.4 Green: the pause watch's threshold already follows the noise floor (design 6, no code change); unit test in `engine/pause.rs`
- [x] 4.5 Red: pause scenarios of "An utterance ends after 0.2 s of silence" with 0.2–0.6 s gaps; see them fail — reverted with 4.6: the requirement is "An utterance ends after 0.3 s of silence", gaps 0.3–0.6 s
- [x] 4.6 Green: `--min-silence-secs` 0.2 in the service default and the app catalog — reverted to 0.3 (design 7): 0.2 lost recorded forms
- [x] 4.7 `just test`; commit

## 5. Service: doubled forms at once

- [x] 5.1 Red: E2E scenarios of "A doubled command needs no pause"; see them fail
- [x] 5.2 Green: two spotter hits of one single-word command within 0.8 s ask Whisper at once and report a doubled form at once; unit tests in `engine/spotted/`
- [x] 5.3 `just test`; commit

## 6. Whisper context

- [x] 6.1 `whisper_args` takes an audio context; the commands service gets `-ac` from its longest clip, in the app catalog and `dev-tools`
- [x] 6.2 Run the voice-sample and voice-control smoke tests; revert when accuracy drops (design 11) — reverted: with `-ac 1100` four of the six real-service confirmation tests heard no command, 6 of 6 passed with `-ac 0` on the same services
- [x] 6.3 `just test`; commit

## 7. Confirmation switch

- [x] 7.1 Red: component scenarios of "Commands can run without the Whisper confirmation" and "A command carried out without confirmation records whether Whisper agreed"; settings scenarios of "Voice control has a switch for the Whisper confirmation"; see them fail
- [x] 7.2 Green: `[voice_control] confirm_commands`, Settings switch, `select` field and protocol version bump
- [x] 7.3 Green: service reports unconfirmed commands at once and sends the `heard` verdict; the command history entry records it
- [x] 7.4 Green: the app logs the verdict and "unconfirmed" in the start line
- [x] 7.5 E2E: "Start start without confirmation starts at once"
- [x] 7.6 `docs/ubiquitous-language.md`: unconfirmed command, Whisper verdict
- [x] 7.7 `just test`; commit

## 8. Bounds and after numbers

- [x] 8.1 Red: cue bounds 0.5 s for a doubled form, 0.7 s for every other form (the 0.2 s pause was reverted, design 7), and the overlay scenario in `voice_control.feature`; run `just e2e-cues`
- [x] 8.2 Green: steps 4–7 within the bounds; revert any step that adds a false start or loses accuracy (design 11)
- [x] 8.3 Record the after numbers per step in `design.md`
- [x] 8.4 `just test`; commit
- [x] 8.5 No rule is renamed: the 0.5 s bound holds only for doubled forms, so "within 0.7 s" keeps its heading (design 7)

## 9. Version and check

- [x] 9.1 Bump `version` in `app/Cargo.toml` to 0.40.0 (or the next free minor)
- [x] 9.2 `just test`; `just check` once; commit — `just full-check` on the final commit instead (kit verify)
- [x] 9.3 Manual run by the user: say "start start" 10 times with confirmation on and 10 with it off, paste the log; median at most 350 ms, one in ten at most 500 ms; recording, overlay and sound work — skipped: the user chose to merge without a manual run (2026-10-27)
