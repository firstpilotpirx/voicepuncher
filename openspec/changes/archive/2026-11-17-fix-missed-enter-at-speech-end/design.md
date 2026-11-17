## Context

Command spotting runs two streaming recognizers (English Nemotron 0.6B, Russian small zipformer) per utterance behind a Silero gate (`command-spotting/src/streaming/session/`). An utterance closes after 0.3 s of silence; both streams then get 1 s of silence and `input_finished`, and the last decode is decided at `Moment::Closing`. The four misses of 2026-11-16..17 (proposal) and one fast enter of the same days that worked live share one shape: a fast "enter" glued to the last word, at the end of the utterance. Replaying them through `streaming-bench`:

| Experiment | Clips heard (of 5: 4 misses, 1 that worked live) |
|---|---|
| as today | 1 (only the clip that worked live) |
| hotwords, `modified_beam_search`, score 1.5–5 | 2 more |
| the last 1.0 s of speech alone, in fresh streams | 4 |
| the last 0.8 s and 1.0 s, each alone (the fix) | 5 |

## Goals / Non-Goals

**Goals:** report the fast trailing "enter" in each of the five clips (4 misses, 1 that worked live); keep every ordinary-word test green; keep the delay of a command the running decode hears unchanged.

**Non-Goals:** a Whisper fallback (the user rejected it for command spotting); the cut point of a stopped dictation (`fix-lost-words-before-voice-commands`); a bigger Russian model.

## Decisions

### 1. Second look at the last second of speech

Once every main decoder of a closing utterance finished and the utterance sent no report, the session's `collect_closing` starts its second look (`Utterance::look_again`, `session/utterance/second_look.rs`): per window and per recognizer a fresh stream on its own `Decoder` thread, fed lead silence (`LEAD_SECS`), the utterance's own audio from `speech_end - window` (not before its start) to `speech_end + 0.1 s`, `CLOSE_SILENCE_SECS` of silence, then `finish`. Two windows, 0.8 s and 1.0 s: of the five clips 0.8 s hears all, 1.0 s four, 0.6 s three, 0.9 s four, so one alone is fragile. When every glance finished, each one's words (times mapped back to the stream: window start + time − lead) are decided at `Moment::Closing`; the first finding that is "press enter" alone goes out through the utterance's `report`, unless the next utterance owns it (`handover`). Only "enter": the bug is about it, and a "stop" a window of Russian speech is misheard as would end a dictation. `finished()` waits for the second look; an utterance that sent a report drops it at once. The command history entry keeps each recognizer's second-look text beside its own (named `<recognizer> second look <window> s`), so an entry whose command only the second look heard is not counted as missed.

*Why lazy:* a command the running decode heard keeps its delay and costs no extra decode; only an utterance without a command pays about 0.1–0.4 s more before it is done. *Why short windows:* 1.5 s and 2.0 s heard 2 of 5: a longer window lets the preceding words pull the model off again. *Alternative rejected:* decoding the window at every close in parallel — lower delay for a miss, but an extra decode of every utterance, all day.

### 2. A last "энтер…" word counts

`timed_words` respells the last word of a respelled recognizer's text with `respell_last_word`: `respell_word` first, then, when the word starts with the "enter" sounds opening in "э" (э, н, т/д, е/э/о/а optional, р) and more letters follow, `["enter"]`. Only "э": "интер…" opens many ordinary words ("интересно", "интернет", "интерфейс"). Only the last word: inside speech such a word stays speech; while the utterance runs, the existing settle rules keep a last word that more speech follows from counting.

### 3. No hotwords

The user asked for hotwords too (sherpa's contextual biasing: `modified_beam_search`, 4 active paths, score 2.0, `▁э н тер` for Russian, `▁E n ter` and `▁ent er` for English). Measured on the bench dataset at the fast pace, they heard 109 of 121 clips against 110 without, broke two command chains of session 4 ("stop enter start" heard with two extra commands, "cancel enter start" without its enter), and decoded 18 % slower (0.773 against 0.654 s per second of audio). Alone they heard 2 of the 4 missed enters; the second look hears all 5 clips without them. So they are left out: the decoding stays `greedy_search`.

### 4. A command word with a repeated word after it

`without_trailing_command` (`app/src/core/domain/command_words/`) also removes a trailing command followed by a repeat (1–3 words) of the words right before the command: the chunk cut at the command heard those words again after the previous chunk had already heard past the command ("…вообще? Enter! вообще."). The repeat and the command go; the sentence end before the command stays.

## Test levels

| Scenario | Level |
|---|---|
| A fast enter glued to the last word presses Return | integration (`command-spotting/tests/it/streaming/glued_enter.rs`) and E2E `.feature`, `@voice_samples` (`glued-enters` recordings) |
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
- [The second look decodes up to 1.4 s of audio four times after every utterance without a command] → only after the close, on the recognizers' threads; the streaming bench's processing time and the stage budgets of `check` show its cost.
- [A missed enter now comes about 0.1–0.4 s after the close, 0.3 s after the speech] → acceptable: before, it never came; the spec bounds it at 1.5 s from the end of its words on an idle machine, which the unit test of 3.1 checks. The bounds of the running decode (0.6 s idle, 1.2 s busy) stay for every command it hears.

## Migration Plan

None: the service and the app ship together; no protocol change.

## Open Questions

None.
