## 1. The cut falls at the command's capture time

- [x] 1.1 Red: add `app/tests/it/commandspotting_command_times.rs` (listed in `app/tests/it/main.rs`): a stream 0.8 s behind the clock reports "press enter" at 1.78 s and at 3.97 s; its start must be the capture time of that sample; run it with `just test-one`, see it fail; commit it alone as intentionally red
- [x] 1.2 Green: map the command's start to its capture time in `to_capture_times` (`app/src/adapters/secondary/commandspotting/websocket/stream.rs`), keeping the end as is; update the doc comments of `SpottingEvent::CommandsSaid` and `close_dictation`
- [x] 1.3 `just test`, commit

## 2. Whole-word "enter" in the Russian recognizer's words

- [ ] 2.1 Red: respelling tests in `command-spotting/src/streaming/respell/tests.rs`: «линтр», «линтер» are no "enter"; «лентер», «лентарентор» still are
- [ ] 2.2 Red: `decide` tests in `command-spotting/src/streaming/decide/tests/at_once.rs` from the recognizer's words of 2026-11-13 16:31 («…возьму интер» then «…возьму интернет») and 2026-11-12 20:17 («должен линтр проверять»): no report; «тоже учесть интер» reported at the gate's pause and at the close, not before; «…читать энтер» still reported at once; run with `just test-one`, see them fail; commit as intentionally red
- [ ] 2.3 Green: the «л»-opened form of "enter" takes only «э» or «е» (`command-spotting/src/streaming/respell/mod.rs`)
- [ ] 2.4 Green: mark a last word respelled as one "enter" from a Russian word opening with «ин» as waiting (`TimedWord`, `timed_words` in `command-spotting/src/streaming/words.rs`); the at-once rule waits on it (`command-spotting/src/streaming/decide/mod.rs`)
- [ ] 2.5 Qualify "reports it at once" in the "Streaming recognizer" entry of `docs/ubiquitous-language.md` with the «интер» wait
- [ ] 2.6 `just test`, commit

## 3. Version and checks

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.49.3
- [ ] 3.2 `just test`, commit; `just check` once
- [ ] 3.3 Manual checks: none (no hotkey, recording, overlay, paste or permission change)
