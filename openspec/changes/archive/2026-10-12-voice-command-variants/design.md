# Design

## Context

Measured on the user's recording (`~/.voicepuncher/voice-samples/session-1`, 40 labelled commands): the gigaspeech keyword spotter finds 24 in a continuous stream (and only 5 when each utterance is fed alone after digital silence), whatever the threshold (0.05–0.25) and boost (1–3); "start dictation" 2 of 9. The local whisper-server transcribes 35 of 40 exactly and 39 with tolerant matching. No threshold produced a false command on 30 benchmark corpus recordings.

## Decisions

### 1. The service confirms short utterances with the speech recognition service
When a speech segment closes and lasts at most `--max-command-secs` (3 s), the service sends it to the speech recognition service (`--transcription-url`; no default — without it the service runs the keyword spotter alone; the justfile passes the local whisper-server, `http://127.0.0.1:8081/v1`), in English, temperature 0, with the command list as prompt. The audio is cut from a ring buffer with 0.8 s of pre-roll, since the voice activity detector trims the first sound, but never before the end of the previous segment, so the tail of the last utterance does not leak in; 0.2 s of post-roll keeps the last sound. When the transcript is a command phrase of the selected set, the service reports that phrase spanning the whole segment. A keyword spotter phrase still counts as before. If the speech recognition service does not answer (10 s timeout), the service logs it and keeps the keyword spotter alone.

The voice activity detector is reset after each segment, its stream position carried as an offset: without it, quiet speech after a few loud segments was missed.
- Alternatives: lower thresholds or other tokens (measured: no gain); a bigger keyword model or a streaming ASR model (another model to fetch and serve; whisper already runs for dictation).

### 2. A pure phrase matcher decides what a transcript means
`PhraseMatcher` (service crate): lowercases, keeps letters, drops filler words ("the", "a", "an") and a trailing "please", collapses a repeated phrase, maps aliases ("clear text" → "clear all text", "presenter" → "press enter", "star dictation" → "start dictation"), then accepts the nearest phrase of the selected set when the normalized character edit distance of the whole string is at most 0.25 and of every word at most 0.4 (so "cancel the quotation" is "cancel dictation", but "start" alone is nothing). It tries the whole transcript first; when whisper splits it into sentences ("Start. Activation."), every sentence must give the same phrase. Anything else is no command, so "please press enter" or "press enter when you are ready" stay text.

### 3. The recognizer counts one command per utterance
Several phrases in one segment that name the same command (the keyword spotter and whisper both reporting it, or a repeat) yield one command. Phrases of different commands reported before the segment closes yield none. A command is carried out as soon as a phrase fills its closed segment, so a phrase of another command that arrives later cannot undo it; it is only ignored.

### 4. The phrase set switches before the recording starts
On "start dictation" the pipeline asks the service to listen for the dictation phrases first and starts recording after; if the recording fails, it switches back to the standby phrases. Switching after the recording started let a quick "stop dictation" be heard with the standby set and lost.

### 5. Keywords for the new phrases
`keywords.txt` gains "… please" lines for every phrase, "clear text" and "clear text please", and "star dictation".

### 6. Tests on the user's voice and whole sessions
Recordings stay on the machine; the justfile exports `VOICEPUNCHER_VOICE_SAMPLES` when the directory exists. Without it the user-voice test and the `@voice_samples` scenarios are skipped with a `GATED` line on stderr. `voice_samples_it` streams the whole session recording and counts one command per utterance: labelled recordings of the same command less than 0.5 s apart are one utterance (the service ends an utterance after 0.4 s of silence and its detector holds speech a little longer); each utterance must give exactly one command within 1 s of it, and no other command may appear.

A hands-free session E2E runs the real pipeline, command spotting service and speech recognition service, with the microphone, paste and key presses stood in: one microphone stream feeds both the spotter and the recorder, and room noise from the user's recording plays between steps, as a live microphone never goes silent. A pasted sentence must keep at least 60 % of its words — whisper misspells some Russian words of a synthesized voice — and hold no word of a command phrase the sentence lacks. A manifest line per utterance: file, start and end in the session recording, the phrase said, the command, a note; `"unclear": true` marks a recording no human-level listener would label confidently ("cancel this" for a rushed "cancel dictation").

| Spec scenario | Level |
|---|---|
| Press enter please presses Return | unit (matcher); E2E `.feature` (`say`, real service + recognizer) |
| Stop dictation please pastes the dictated text | unit (matcher); E2E `.feature` |
| Saying clear text selects all and deletes | unit (matcher); E2E `.feature` |
| A repeated press enter presses Return once | unit (recognizer, matcher) |
| Every recorded command of the user is recognized | adapter integration (`voice_samples_it`: the session recording streamed through the real service and recognizer, phrase set switched per command) |
| A short Russian phrase is no command | E2E `.feature` (`say -v Milena`, real service with whisper confirmation + recognizer) |
| A hands-free session in a synthesized voice | E2E `.feature` (`HandsFreeSession`) |
| A hands-free session with the user's recorded commands | E2E `.feature` (`@voice_samples`) |
| A hands-free session with long pauses between its steps | E2E `.feature` (`@voice_samples`) |
| (design 1, whisper does not answer) | service integration (`engine_it`) |
| (design 4, phrase set before recording) | unit (pipeline) |

## Risks / Trade-offs

- [Whisper load while listening] → only utterances of at most 3 s go to whisper; during a dictation they compete with chunk transcription for the single-slot server, adding up to a few hundred ms.
- [Whisper forced to English hears a short Russian utterance as a command] → exact-or-near whole-utterance match only; the corpus and Russian `say` phrases check it.
- [The speech recognition service is down] → keyword spotter alone, as before.
