## Context

See proposal.md — Why; `research/README.md` holds the whole bench and how to rerun it for a new model.

## Decisions

**A second engine, chosen at service start.** `command-spotting/src/streaming/` is a new engine picked by `--recognizer streaming` (`keyword-spotting` stays the default, so the scripts and existing tests keep their behaviour). The server drives a `Listening` enum over both sessions; the keyword spotting engine stays untouched. Rewriting it in place would break ~50 requirements' tests at once and leave no fallback.

**Ports.** The session works on `SpeechGate` (Silero `detected()` per 512 samples, gain 1.5 as today) and `StreamingRecognizer` (accept, decode, words with times, reset), so unit tests script both. The sherpa-onnx `OnlineRecognizer` (`=2.0.1`, already a dependency) implements the second; each model loads once per service, one stream per utterance.

**Gate.** The last 0.5 s of audio is kept and replayed into both recognizers at onset, so the gate adds no end latency. After `--min-silence-secs` (0.3 s) without speech the recognizers get 0.5 s more audio, a final decode, and their streams are dropped; speech returning first continues the utterance. Each 100 ms frame is decoded by both recognizers on two scoped threads.

**Decision rules, per recognizer, on every decode.** Tokens become words (`▁` opens a word). A Russian word made only of Cyrillic command patterns and fillers (`research/bench/common.py` `RU_CMD`, `RU_FILL`) is respelled in English ("энтерэнтер" → "enter enter"); other words stay. Then the existing `PhraseMatcher` decides, so forms, mishearings, states and "please" keep one definition:
- `anywhere` (doubled, chained) and the early start (`start_so_far`, `start_at_head`) report at once;
- a whole utterance that is one command (`phrase_in`) reports at once when its words are the phrase itself ("press enter"), else once settled (no new word for 0.2 s of audio), so neither "stop" nor "stop the" of "stop the car" is a command;
- `at_end` (speech, then a command) reports when the utterance ends.
Reports are `Command` events (a whole-utterance command is `Tail`), which the app carries out at once. An utterance keeps the end time of its last report; a finding starting before it is skipped, whichever recognizer found it.

**Trace.** Each utterance records onset, end of speech and close; per recognizer each text change with its stream time, total decode time and slowest frame; each report with its recognizer, time and delay after its last word. On close the service logs one line with them; with a command history it keeps an entry when a final text holds a command word or something was reported: the audio from the replay on and `entry.json` with the trace under `streaming`, beside the usual `reported` and `outcome`.

**App side.** `[voice_control] recognizer = "streaming" | "keyword_spotting"`, default streaming. The service catalog skips the commands Whisper service for streaming and passes `--recognizer streaming --en-model-dir --ru-model-dir`; the model catalog adds the two archives and splits voice control models by recognizer. `dev-tools` builds the same line under `VOICEPUNCHER_COMMAND_RECOGNIZER=streaming`, so the parity test covers it.

## Risks / Trade-offs

- [The pair fired on 13% of non-command utterances in the bench] → the matcher is stricter than the bench's word scan; the trace measures the real rate.
- [Nemotron takes ~70% of a core while speech lasts and 650 MB per copy] → the gate idles it in silence.

## Test levels

| Scenario | Level |
|---|---|
| A command said alone is reported before its pause ends | E2E `voice_control.feature` |
| Silence wakes no recognizer | E2E `voice_control.feature` |
| Speech without a command is no command | E2E `voice_control.feature` |
| A Russian-accented stop is heard as stop | E2E `voice_control.feature` |
| A Russian word that is no command stays speech | E2E `voice_control.feature` |
| A command both recognizers hear is reported once | E2E `voice_control.feature` |
| A doubled command is reported once | E2E `voice_control.feature` |
| A reported command leaves an entry with its timeline | E2E `voice_control.feature` |
| Voice control starts the streaming recognizer by default | component `local_services.feature` |
| The keyword spotter can still be chosen | component `local_services.feature` |
| A cold launch starts the needed services | component, unchanged |
| Voice control needs the command services | unit test of the service catalog, keyword spotter chosen |
| A real launch leaves no service behind at quit | E2E `local_services.feature`, unchanged |
| The app and the scripts start the speech recognition service with the same arguments | unit parity test, unchanged |
| The app and the scripts start the streaming recognizer with the same arguments | unit parity test |
| A changed argument in only one place is caught | unit parity test, unchanged |
| Of two keyword spotting models the newest is used | component `local_services.feature`, keyword spotter chosen |
| The streaming recognizer needs the streaming models | component `model_download.feature` |
| The keyword spotter needs its own models | component `model_download.feature` |

Respelling, decision rules, gate replay and the trace are unit tests in `command-spotting/src/streaming/`.
