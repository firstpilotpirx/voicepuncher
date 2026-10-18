# Tasks

## 1. Timing log and measurement (D1, D5)

- [x] 1.1 Red: unit test in `app/src/adapters/secondary/commandspotting/websocket/heard.rs` — a stream position maps to its capture time after part of the audio was lost
- [x] 1.2 Green: record the capture time of each streamed block in the heard audio
- [x] 1.3 Red: unit tests in `command-spotting/src/protocol.rs` and `outbox.rs` — a confirmed command or phrase carries its timing; events without one serialize as before
- [x] 1.4 Green: fill the timing in `command-spotting/src/engine/confirming.rs`; parse it in the app's command spotting adapter
- [x] 1.5 Red: component scenarios "A voice start logs its timing", "The timing stays right after microphone audio was lost", "A hotkey start logs no voice start timing" in `app/tests/features/component/voice_control.feature` with steps in `app/tests/steps/voice_control.rs`
- [x] 1.6 Green: log the timing line at the start cue in `app/src/core/application/pipeline/voice_control.rs`
- [x] 1.7 Red (reproduction, committed alone): E2E scenarios for every "gives its cue at once" scenario and the user's recorded starts in `e2e-test/tests/features/voice_control.feature`, steps measuring the cue from the command's end sample
- [x] 1.8 Run the latency scenarios and record today's numbers per placement in `design.md` § Measurements
- [x] 1.9 `just test`, commit

## 2. Check at the spotted start (D2, D3)

- [x] 2.1 Red: `command-spotting/tests/engine_it.rs` — "start start" followed at once by speech is reported within 0.6 s of its end
- [x] 2.2 Red: unit test in `command-spotting/src/engine/confirming.rs` — the head check takes "start start" and "enter start"
- [x] 2.3 Green: the head check applies the anywhere matcher
- [x] 2.4 Green: a spotted "start" or "restart" in a running utterance asks a check 0.35 s after its end in `command-spotting/src/engine.rs`
- [x] 2.5 Red: unit test — "start" heard alone by the spotted check is taken only from the second check 0.3 s later
- [x] 2.6 Green: the second check
- [x] 2.7 Red: unit test — a spotted check falling due while another check waits is asked when that one answers, and never two checks wait
- [x] 2.8 Green: defer the spotted check
- [x] 2.9 Run `command-spotting/tests/placed_commands_it.rs` and the false-start E2E scenarios; `just test`, commit
- [x] 2.10 Red: `engine_it` — a short "start" or "restart" said in a running utterance is spotted and asks the spotted check, and is never reported as a phrase on its own
- [x] 2.11 Green: trigger-only short keywords for "start" and "restart" in `command-spotting/keywords.txt` and `keywords.rs`
- [x] 2.12 Find why "restart" said on its own is heard as another word since group 2, and fix it; record the latency after group 2 again; `just test`, commit

## 3. Segment confirmations beside checks (D4)

- [x] 3.1 Red: unit test in `command-spotting/src/worker.rs` — a segment confirmation is answered while a check hangs
- [x] 3.2 Green: a second confirmation thread for checks per connection
- [x] 3.3 `just test`, commit

## 4. Restart without waiting for the dropped dictation (D6)

- [x] 4.1 Red: unit test in `app/src/core/application/pipeline.rs` — "restart" gives its start cue before the live worker returns its text
- [x] 4.2 Green: cancel without flushing; the new live session takes the transcriber back from the old worker
- [x] 4.3 `just test`, commit

## 5. Measure again

- [x] 5.1 Run the latency scenarios; record the numbers per placement in `design.md` § Measurements
- [x] 5.2 Only if a lone start misses the bar: Red then Green for confirming it during its pause — not kept: with the room fed in real time the recorded lone starts give the cue within 0.7 s in 5 of 6, and the clip heard during the pause, 0.03–0.07 s shorter than the segment's, moved whisper's word times in the user's session-1 (`voice_samples_it` failed twice); reverted
- [x] 5.3 Only if the cue stage exceeds 100 ms: Red then Green for showing the overlay before the recorder opens — not needed: the synthesized "start start" gets its cue 213 ms after its words, hearing and confirmation included; the app's `cue=` of about 315 ms is logged after the stand-in recorder opens (`design.md` § Measurements, after group 5)
- [x] 5.4 `just test`, commit
- [x] 5.5 Feed the room after a timed command in real time, so its pause reaches the service when it was said; report where the command's words end in the stream and, on `RUST_LOG`, the app's `voice start:` timing
- [x] 5.6 Red then Green: while "start" is listened for, search the start of a running utterance 1.0 s after it began for a command opening speech (as the first spotted check does), since the keyword spotter misses most of the user's "start" and the head check comes only at 1.5 s

## 6. Docs

- [x] 6.1 Add *start cue* to `docs/ubiquitous-language.md`
- [x] 6.2 Add the timing line to the healthy log in `AGENTS.md` § How to verify a change and `README.md` § Release checklist
- [x] 6.3 `just test`, commit

## 7. Release

- [x] 7.1 Bump `version` in `app/Cargo.toml` to 0.18.2
- [x] 7.2 `just check`, commit — run as `just full-check`, which holds every stage of `check`
- [x] 7.3 Manual checks by the user (at the manual run stage of the pull request): "start start" followed at once by speech, and "start" said alone, each shows the timing line within 0.7 s and the start sound and overlay at once; "restart" after a long dictation
