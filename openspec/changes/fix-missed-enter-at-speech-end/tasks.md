## 1. A last "энтер…" word counts

- [ ] 1.1 Red: unit tests in `command-spotting/src/streaming/words.rs`: the Russian text "там отмечать энтерф" reads its last word as "enter"; "энтерф там отмечать", "интересно" and "интерфейс" at the end stay speech; `just test-one` fails
- [ ] 1.2 Green: `respell_last_word` in `command-spotting/src/streaming/respell/`, used by `timed_words` for the last word of a respelled recognizer; the same `just test-one` passes
- [ ] 1.3 `just test` and commit

## 2. A trailing command with a repeated word after it is not pasted

- [ ] 2.1 Red: unit test in `app/src/core/domain/command_words/`: "Ее типа нет, что ли, вообще? Enter! вообще." without its trailing command is "Ее типа нет, что ли, вообще?"; a sentence whose words after a command word repeat nothing keeps them; `just test-one` fails
- [ ] 2.2 Green: `without_trailing_command` also cuts at a command followed by a repeat (1–3 words) of the words right before it; the same `just test-one` passes
- [ ] 2.3 `just test` and commit

## 3. Second look at the last second of speech

- [ ] 3.1 Red: session unit tests (`command-spotting/src/streaming/session/tests/second_look.rs`, scripted recognizers whose fresh stream hears what the window holds): an utterance whose running decode hears no command but whose last second holds "enter" reports "press enter" once, at its time in the utterance, within 1.5 s of the end of its words; the history entry keeps the second-look texts; an utterance that reported "enter" opens no second look; speech without a command reports nothing; `just test-one` fails
- [ ] 3.2 Green: `SecondLook` in `command-spotting/src/streaming/session/utterance/`: prepared at `begin_close`, fed once the main decoders finished with nothing reported, decided at `Moment::Closing`, reported through `report`; `finished()` waits for it; the same `just test-one` passes
- [ ] 3.3 Red: integration test `command-spotting/tests/it/streaming/glued_enter.rs` (`needs_streaming_models::needs_voice_samples`): each recording in `$VOICEPUNCHER_VOICE_SAMPLES/glued-enters/` (the missed clips of 2026-11-16..17) reports "press enter" within 1.5 s of the end of its words; it fails before the group's code (checked on the commit before 3.2)
- [ ] 3.4 Green: the integration test passes; the streaming bench over the bench dataset shows no new false alarm
- [ ] 3.5 `just test` and commit

## 4. Hotwords

- [ ] 4.1 Red: `needs_streaming_models` test that every piece of each recognizer's hotwords is a token of its model's `tokens.txt`; unit test that the hotwords of "ru" and "en" are the lines of design § 3; `just test-one` fails
- [ ] 4.2 Green: `SherpaModel::load` sets `modified_beam_search`, 4 active paths, score 2.0 and the recognizer's `hotwords_buf`; the same `just test-one` passes; the streaming bench shows no new false alarm and no slower decode beyond the stage budget
- [ ] 4.3 `just test` and commit

## 5. E2E and release

- [ ] 5.1 Rule "A fast enter glued to the end of speech counts" in `e2e-test/tests/features/voice_control.feature`: "A fast enter enter glued to the last word presses Return" and "Every enter the running decode missed in the user's history is heard" (`@voice_samples`, the `glued-enters` recordings), "Speech without a command ending it stays speech in the second look" and "An utterance that reported its command reports it once" (`@smoke`)
- [ ] 5.2 Bump `version` in `app/Cargo.toml` to 0.49.2
- [ ] 5.3 `just test` and commit
