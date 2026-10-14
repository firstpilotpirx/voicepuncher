# Design

## Context

The service reported a phrase in two ways: the keyword spotter placed a phrase in the stream, and for a closed utterance of at most 3 s it asked the speech recognition service (whisper-server) what was said and reported a phrase spanning the whole utterance. The app's `VoiceCommandRecognizer` keeps such a phrase only when it fills its utterance (lead ≤ 0.3 s, tail ≤ 0.5 s). Commands said without a pause were joined into a long utterance and lost. Whisper recognizes the user's accent far better than the keyword spotter, so the new forms are found by whisper on windows of the utterance, using its word times.

## Decisions

### 1. The matcher reads every placement
`PhraseMatcher` (service) keeps `phrase_in` (the whole transcript is a command) and gains, on the words of a transcript:
- `at_end`: the last words are a command phrase or short form (with "please"); its last word must be the last word said, so "clear the…" is no "clear".
- `start_at_head`: "start dictation" or "start" followed by other words; `start_so_far`: "start" alone as everything heard so far of an utterance still being spoken.
- `anywhere`: a doubled form ("stop stop") or a chain of two or three different commands, each accepted in the state the one before leads to; "start" only last; "restart" in no chain; "and restart" closing a chain is the user's quick "enter start", as whisper hears it. A "start" followed at once by another command word ("start stop") opens no dictation.
The matcher knows the state machine (standby: start, enter, clear; dictating: stop, cancel, restart, enter — "enter" there ends the dictation first) and the current state from the phrase set the app selected. A one-word form must be said exactly; a longer one may differ by ≤ 0.4 per word, and of the forms said the longest, then the closest, wins ("restart dictation" is close to "start dictation" too). Every command has a one-word short form: start, stop, cancel, restart, enter, clear.

### 2. When the service asks the speech recognition service
whisper-server answers `verbose_json` with word times (`timestamp_granularities[]=word`); pieces of a word are joined.
- **Closed utterance:** a short one (≤ 3 s, with 0.8 s pre-roll) is checked whole first; its command is placed from its first word, since the voice activity detector notices speech up to 0.7 s late and a recording cut there kept the command's first word. Otherwise its last 2.5 s → `anywhere`, then `at_end`.
- **Closing check, second chance:** when neither finds a command, a command that ended the last running window counts when it ends within 0.6 s of the utterance's end — the closing window, cut elsewhere, can mishear it ("at ten enter" as "at 10am").
- **Running utterance:** at 1.5 s, its start → `start_at_head` or `start_so_far` (standby only); from 2 s on, every 1 s, the last 2.5 s → `anywhere`. A command found is not reported again by later windows or by the closing check (`reported_until`).
- **Missed utterance:** a keyword-spotter phrase that no speech segment took within 1 s (quiet speech in noise) is checked on a window around it; when whisper hears only the command, it counts.
- Requests are sequential per connection (~0.2–0.4 s each on Metal); audio queues meanwhile.

### 3. A new event carries a command that needs no pause rule
`ServerEvent::Command { commands, start, end, placement }` (`tail`, `head`, `anywhere`): one to three phrases and the span of their words in the stream. The app's recognizer turns it into one to three `SpokenCommand`s at once, in order, and ignores a later phrase of the same words (`carried_out_until`). `Phrase` keeps the pause rule.

### 4. Cutting the dictation and its text
- **Stop at the end or anywhere:** the recording is cut at the command's first word; the text also loses trailing command words (`without_trailing_command`), in case the cut lands a word late.
- **Start at the head:** the command spotting adapter keeps the last 5 s of streamed audio (`CommandSpotter::heard_since`); the recorder starts with the audio heard after "start" as lead-in (`AudioRecorder::start_with_lead_in`, upsampled to the microphone rate), and the text loses a leading "start" (`without_leading_start`).
- **Restart:** cancel, then start a new recording.

### 5. Shorter pauses
`--min-silence-secs` defaults to 0.3 and the shortest speech the detector keeps to 0.15 s (a short "start" lasts about 0.2 s).

## Test plan

| Spec scenario | Level |
|---|---|
| Stop right after the last dictated word | unit (matcher `at_end`, text strip); component `.feature`; E2E `.feature` (`say`) |
| Enter right after other speech | unit (matcher); E2E `.feature` |
| A command word inside a sentence is still no command | unit (matcher); E2E `.feature` |
| Start followed by the dictation at once | unit (matcher `start_at_head`, `start_so_far`, recorder lead-in); component `.feature`; E2E `.feature` |
| Enter followed by speech is no command | unit (matcher); E2E `.feature` |
| Stop stop in the middle of speech | unit (matcher `anywhere`, text strip); component `.feature`; E2E `.feature` |
| Enter enter in the middle of speech | unit (matcher); E2E `.feature` |
| Cancel cancel in the middle of speech | component `.feature`; E2E `.feature` |
| Stop enter pastes and sends | unit (matcher, recognizer); component `.feature`; E2E `.feature` |
| Stop enter start sends and starts the next dictation | unit (matcher); component `.feature`; E2E `.feature` |
| Cancel clear discards and clears | component `.feature`; E2E `.feature` |
| A chain the states do not allow is no command | unit (matcher) |
| Stop clear enter | unit (matcher); component `.feature`; E2E `.feature` |
| Stop after a short breath | service integration (`placed_commands_it`); E2E `.feature` |
| Saying restart drops the dictation and starts a new one | component `.feature`; E2E `.feature` |
| Restart restart in the middle of speech | unit (matcher); E2E `.feature` |
| A short form is recognized as its command | unit (matcher); E2E `.feature` (all six, with and without "please") |
| Cancel alone discards a dictation | E2E `.feature`; the user's voice (`session-3`) |
| A repeated press enter presses Return once | unit (recognizer, matcher) |
| Every recorded command of the user is recognized | adapter integration `voice_samples_it` (`session-1`, repeats within 0.3 s grouped) |
| A short Russian phrase is no command | E2E `.feature` |
| (the forms the user recorded) | adapter integration `voice_forms_it`: `session-2`, `session-3` and `session-4`, 67 checked stretches (one more marked unclear) — short forms, "please", doubled forms, commands at the end and "start" at the head, two-command chains, "cancel" alone, speech that is no command — each streamed alone in its state, with app-like phrase-set switching. `session-4` adds three-command chains, "enter" during a dictation, a breath before the command, "cancel please", "restart please" and "start stop" |
| Saying enter during a dictation pastes and sends | unit (pipeline); component `.feature`; E2E `.feature` |
| Stop dictation without a dictation is ignored | component `.feature`; E2E `.feature` (standby, synthesized) |
| Clear during a dictation is ignored | component `.feature` |
| Only the speech after a restart is pasted | component `.feature` |
| (pipeline) | unit: head start records with the lead-in heard since the command; restart plays cancel then start |

## Risks / Trade-offs

- [A sentence that ends in a command word ("please don't stop", then a pause) ends the dictation] → only with a pause after it and in the state that accepts it; the benchmark corpus sample and the user's recordings give no such command.
- [Whisper on short windows hallucinates a doubled word] → both words must be heard; the corpus sample checks it in both states.
- [Load on whisper-server] → one request per closed utterance, one at 1.5 s and one per second after 2 s of speech; dictation chunks can wait a few hundred ms. E2E scenarios run at once and wait up to 4 s of quiet for late answers.
- [The voice activity detector misses a quiet short command] → the keyword spotter's phrase is confirmed on its own window.
