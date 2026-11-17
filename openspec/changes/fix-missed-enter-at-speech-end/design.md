## Context

Command spotting runs two streaming recognizers (English Nemotron 0.6B, Russian small zipformer) per utterance behind a Silero gate (`command-spotting/src/streaming/session/`). An utterance closes after 0.3 s of silence; both streams then get 1 s of silence and `input_finished`, and the last decode is decided at `Moment::Closing`. The five misses of 2026-11-16..17 (proposal) share one shape: a fast "enter" glued to the last word, at the end of the utterance. Replaying them through `streaming-bench`:

| Experiment | Misses heard (of 5) |
|---|---|
| as today | 1 (only the clip that worked live) |
| hotwords, `modified_beam_search`, score 1.5–5 | 2 more |
| the last 1.0 s of speech alone, in fresh streams | 4 |

## Goals / Non-Goals

**Goals:** report the fast trailing "enter" in each of the five clips; keep every ordinary-word test green; keep the delay of a command the running decode hears unchanged.

**Non-Goals:** a Whisper fallback (the user rejected it for command spotting); the cut point of a stopped dictation (`fix-lost-words-before-voice-commands`); a bigger Russian model.

## Decisions

### 1. Second look at the last second of speech

At `begin_close` the utterance prepares a `SecondLook`: one fresh stream per recognizer (each on its own `Decoder` thread) and the window of its own audio from `speech_end - 1.0 s` (not before the utterance's start) to `speech_end + 0.1 s`. The window is fed only once every main decoder finished and the utterance sent no report: lead silence (`LEAD_SECS`), the window, `CLOSE_SILENCE_SECS` of silence, then `finish`. When every second-look decoder finished, each recognizer's words (times mapped back to the stream: window start + time − lead) are decided at `Moment::Closing` with nothing reported; the first finding goes out through the utterance's `report`, unless the next utterance owns it (`handover`). `finished()` waits for the second look; an utterance that sent a report drops it at once. The command history entry keeps each recognizer's second-look text beside its own (named `<recognizer> second look`), so an entry whose command only the second look heard is not counted as missed.

*Why lazy:* a command the running decode heard keeps its delay and costs no extra decode; only an utterance without a command pays about 0.1–0.4 s more before it is done. *Why 1 s:* the bench heard 4 of 5 with 1.0 s, 2 of 5 with 1.5 s and 2.0 s: a longer window lets the preceding words pull the model off again. *Alternative rejected:* decoding the window at every close in parallel — lower delay for a miss, but an extra decode of every utterance, all day.

### 2. A last "энтер…" word counts

`timed_words` respells the last word of a respelled recognizer's text with `respell_last_word`: `respell_word` first, then, when the word starts with the "enter" sounds opening in "э" (э, н, т/д, е/э/о/а optional, р) and more letters follow, `["enter"]`. Only "э": "интер…" opens many ordinary words ("интересно", "интернет", "интерфейс"). Only the last word: inside speech such a word stays speech; while the utterance runs, the existing settle rules keep a last word that more speech follows from counting.

### 3. Hotwords

`SherpaModel::load` takes the hotwords of its recognizer: Russian `▁э н тер` ("энтер"), English `▁E n ter` and `▁ent er` ("Enter", "enter"), given as `hotwords_buf` (no file to ship), with `modified_beam_search`, 4 active paths, score 2.0. The pieces are the models' own tokens (`tokens.txt`); a missing piece makes sherpa reject the hotword, which the load test catches.

### 4. A command word with a repeated word after it

`without_trailing_command` (`app/src/core/domain/command_words/`) also removes a trailing command followed by a repeat (1–3 words) of the words right before the command: the chunk cut at the command heard those words again after the previous chunk had already heard past the command ("…вообще? Enter! вообще."). The repeat and the command go; the sentence end before the command stays.

## Test levels

| Scenario | Level |
|---|---|
| A fast enter enter glued to the last word presses Return | integration (`command-spotting/tests/it/streaming/glued_enter.rs`) and E2E `.feature`, `@voice_samples` (`glued-enters` recordings) |
| Every enter the running decode missed in the user's history is heard | integration (`command-spotting/tests/it/streaming/glued_enter.rs`) and E2E `.feature`, `@voice_samples` |
| Speech without a command ending it stays speech in the second look | unit (`command-spotting/src/streaming/session/tests/second_look.rs`) and E2E `.feature`, `@smoke` |
| An utterance that reported its command reports it once | unit (`command-spotting/src/streaming/session/tests/second_look.rs`) and E2E `.feature`, `@smoke` |
| A Russian-accented stop is heard as stop | E2E `.feature` (existing) |
| A Russian word that is no command stays speech | E2E `.feature` (existing) |
| A glued enter enter written by the Russian recognizer counts as the doubled command | unit (existing, `command-spotting/src/streaming/session/tests/`) |
| A mangled enter written by the Russian recognizer counts as enter | unit (existing, `command-spotting/src/streaming/session/tests/`) |
| Every glued or mangled enter from the history reads as the command | unit (existing, `command-spotting/src/streaming/respell/tests.rs`) |
| An ordinary Russian word with the letters of enter stays speech | unit (existing, `command-spotting/src/streaming/respell/tests.rs`) |
| A last word starting with the sounds of enter counts as enter | unit (`command-spotting/src/streaming/words.rs`) |
| A word starting with the sounds of enter inside speech stays speech | unit (`command-spotting/src/streaming/words.rs`) |
| Stop right after the last dictated word | component `.feature` (existing) |
| Enter right after other speech | E2E `.feature` (existing) |
| Stop after a long dictation counts without waiting for a pause | E2E `.feature` (existing) |
| A command word followed by a repeat of the words before it is not pasted | unit (`app/src/core/domain/command_words/`) |

macOS `say` cannot be made to glue a fast "enter" to a word as the user does; the real recordings are the user's command history clips, copied into `~/.voicepuncher/voice-samples/glued-enters/` (never committed), so those tests are `needs_voice_samples`.

## Risks / Trade-offs

- [The second look reports a command in speech that has none] → it decides with the same placement rules at `Moment::Closing`, only when nothing was reported; the ordinary-words scenarios and the bench's false alarms are checked.
- [Beam search costs more CPU than greedy] → measured with `streaming-bench` before and after; the stage budgets of `check` catch a slower voice control stage.
- [A missed enter now comes about 0.1–0.4 s after the close, 0.3 s after the speech] → acceptable: before, it never came; the spec bounds it at 1.5 s from the end of its words on an idle machine, which the unit test of 3.1 and the integration test of 3.3 check. The bounds of the running decode (0.6 s idle, 1.2 s busy) stay for every command it hears.

## Migration Plan

None: the service and the app ship together; no protocol change.

## Open Questions

None.
