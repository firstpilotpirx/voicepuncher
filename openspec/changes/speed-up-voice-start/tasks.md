# Tasks

## 1. Timing log and measurement (D1, D5)

- [ ] 1.1 Red: unit test in `app/src/adapters/secondary/commandspotting/websocket/heard.rs` — a stream position maps to its capture time after part of the audio was lost
- [ ] 1.2 Green: record the capture time of each streamed block in the heard audio
- [ ] 1.3 Red: unit tests in `command-spotting/src/protocol.rs` and `outbox.rs` — a confirmed command or phrase carries its timing; events without one serialize as before
- [ ] 1.4 Green: fill the timing in `command-spotting/src/engine/confirming.rs`; parse it in the app's command spotting adapter
- [ ] 1.5 Red: component scenarios "A voice start logs its timing", "The timing stays right after microphone audio was lost", "A hotkey start logs no voice start timing" in `app/tests/features/component/voice_control.feature` with steps in `app/tests/steps/voice_control.rs`
- [ ] 1.6 Green: log the timing line at the start cue in `app/src/core/application/pipeline/voice_control.rs`
- [ ] 1.7 Red (reproduction, committed alone): E2E scenarios for every "gives its cue at once" scenario and the user's recorded starts in `e2e-test/tests/features/voice_control.feature`, steps measuring the cue from the command's end sample
- [ ] 1.8 Run the latency scenarios and record today's numbers per placement in `design.md` § Measurements
- [ ] 1.9 `just test`, commit

## 2. Check at the spotted start (D2, D3)

- [ ] 2.1 Red: `command-spotting/tests/engine_it.rs` — "start start" followed at once by speech is reported within 0.6 s of its end
- [ ] 2.2 Red: unit test in `command-spotting/src/engine/confirming.rs` — the head check takes "start start" and "enter start"
- [ ] 2.3 Green: the head check applies the anywhere matcher
- [ ] 2.4 Green: a spotted "start" or "restart" in a running utterance asks a check 0.35 s after its end in `command-spotting/src/engine.rs`
- [ ] 2.5 Red: unit test — "start" heard alone by the spotted check is taken only from the second check 0.3 s later
- [ ] 2.6 Green: the second check
- [ ] 2.7 Red: unit test — a spotted check falling due while another check waits is asked when that one answers, and never two checks wait
- [ ] 2.8 Green: defer the spotted check
- [ ] 2.9 Run `command-spotting/tests/placed_commands_it.rs` and the false-start E2E scenarios; `just test`, commit

## 3. Segment confirmations beside checks (D4)

- [ ] 3.1 Red: unit test in `command-spotting/src/worker.rs` — a segment confirmation is answered while a check hangs
- [ ] 3.2 Green: a second confirmation thread for checks per connection
- [ ] 3.3 `just test`, commit

## 4. Restart without waiting for the dropped dictation (D6)

- [ ] 4.1 Red: unit test in `app/src/core/application/pipeline.rs` — "restart" gives its start cue before the live worker returns its text
- [ ] 4.2 Green: cancel without flushing; the new live session takes the transcriber back from the old worker
- [ ] 4.3 `just test`, commit

## 5. Measure again

- [ ] 5.1 Run the latency scenarios; record the numbers per placement in `design.md` § Measurements
- [ ] 5.2 Only if a lone start misses the bar: Red then Green for confirming it during its pause
- [ ] 5.3 Only if the cue stage exceeds 100 ms: Red then Green for showing the overlay before the recorder opens
- [ ] 5.4 `just test`, commit

## 6. Docs

- [ ] 6.1 Add *start cue* to `docs/ubiquitous-language.md`
- [ ] 6.2 Add the timing line to the healthy log in `AGENTS.md` § How to verify a change and `README.md` § Release checklist
- [ ] 6.3 `just test`, commit

## 7. Release

- [ ] 7.1 Bump `version` in `app/Cargo.toml` to 0.15.4
- [ ] 7.2 `just check`, commit
- [ ] 7.3 Manual checks by the user: "start start" followed at once by speech, and "start" said alone, each shows the timing line within 0.7 s and the start sound and overlay at once; "restart" after a long dictation
