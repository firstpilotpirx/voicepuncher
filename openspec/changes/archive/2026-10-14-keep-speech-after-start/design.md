# Design

## Context

With voice control on, the microphone is captured all the time for the command spotting service. The WebSocket adapter streams it, resampled to 16 kHz mono, and keeps the last 5 s of that sound in `HeardAudio`. `CommandSpotter::heard_since(since)` hands that sound out. The positions in it count samples from the capture's start, the same clock the service uses to place a command (`SpokenCommand::ends_at`).

A dictation records with its own microphone stream (`CpalAudioRecorder`), opened when the dictation starts. Only `VoiceCommand::StartDictation` with `CommandPlacement::Head` uses the heard sound today. `carry_out` takes `heard_since(ends_at)` and passes it to `AudioRecorder::start_with_lead_in`, which opens the stream and then puts the sound at the start of the buffer. Every other start (`Alone`, `Anywhere`, `Tail`, a chain ending in "start") and `RestartDictation` calls `start_recording()` with no lead-in.

Two gaps follow:

1. **No lead-in for most starts.** The speech between the command's end and the recorder's start is lost. For a doubled "start start" in a running utterance, the service reports the command only at its first window search, 2 s into the utterance plus the speech recognition time. Even for a command said alone, the report waits for the 0.3 s pause and a speech recognition round trip.
2. **Lead-in taken too early.** For `Head`, the heard sound is taken before `recorder.start()` opens the device. That takes about 0.1–0.3 s on macOS, more with a Bluetooth microphone, and nothing records that time.

The user asked that the dictation start from the sound buffer itself, not from a transcript of it. The heard sound is raw 16 kHz audio and is transcribed together with the rest of the recording.

## Goals / Non-Goals

**Goals:**

- Every dictation started by voice begins at the end of the command's words: "start" in any placement, a chain ending in "start", and "restart".
- No gap and no repeated audio between the heard sound and the recorder's own audio.
- A leading "start" (also a doubled one) that the lead-in carries into the transcript is not pasted.

**Non-Goals:**

- Reporting "start start" sooner, so the overlay appears earlier.
- Recording the whole dictation from the command spotting stream.
- The hotkey's start.
- Making the clock of the stop cut (`truncate_at`) drift-proof.

## Decisions

### D1. Every voice start passes the command's end as the lead-in start

`carry_out` passes `spoken.ends_at` for `StartDictation` in every placement and for `RestartDictation`, which cancels and then starts with the same lead-in start. `start_recording_from` takes `Option<Instant>`, the moment the dictation begins, instead of a ready `AudioBuffer16k`. The hotkey passes `None`.

For a command said alone, the lead-in holds the pause after it, and room sound before speech costs nothing. Alternative: lead-in only for placements with speech after the command (`Head`, `Anywhere`). Rejected, because a command said alone followed by a pause shorter than the time to report it ("start", 0.5 s, sentence) loses speech just the same.

### D2. The lead-in is taken once the recorder runs

`start_recording_from` now runs these steps in order:

1. `recorder.start()`
2. `heard_since(since)`
3. `recorder.begin_with(lead_in)`
4. `start_live_chunks`

The heard sound then ends where the recorder's own sound begins, give or take one capture callback (about 10 ms), and no device opening falls between them. `AudioRecorder::start_with_lead_in` is replaced by `begin_with(lead_in)`, which puts sound heard before the recording started at its beginning; the default does nothing. The cpal adapter's existing splice-and-shift code moves there unchanged. Live chunks start after the splice, so the live watcher sees the lead-in as the start of the buffer.

The join is placed by when the calls run, not by mapping the recorder's start `Instant` onto the heard sample clock. The heard sound counts samples, the recorder counts wall time, and their clocks drift apart by tens of ppm over a day of listening. "Everything held right now" is the same moment for both streams whatever the drift.

Alternative: record the whole voice dictation from the command spotting stream and never open a second stream. Rejected for this fix. The recorder also feeds live chunks, the audio level for the overlay, and the hotkey, so it would be a larger change of its own.

### D3. Heard sound held for 10 s

`HELD_SECS` goes from 5 s to 10 s. During a dictation, the service's speech recognition requests for "restart" queue behind the dictation's live chunks at the speech recognition service, so a report can come seconds late. 10 s of 16 kHz `f32` is 640 KB.

### D4. Leading command words in the text

`command_words.opening` holds the command that started the dictation ("start" or "restart") for every voice start whose placement is not `Alone`. `Alone` takes its end from the voice activity detector's segment end, which comes after the last word, so no command word leaks. Placements whose end comes from word timestamps may leak the tail of the command. This mirrors the existing `closing = placement != Alone` rule for "stop". `without_leading_command(command, text)` replaces `without_leading_start` and removes only the opening command's own forms, also doubled: "Start, start, the build…" → "The build…" after "start start", "Restart, lunch…" → "Lunch…" after "restart restart"; a "Restart the server" that opens a dictation "start start" began stays.

### Test levels

| Spec scenario | Level |
|---|---|
| Start start followed by the dictation at once | component `.feature` (the scripted service reports "start start" only after the sentence began), E2E `.feature` (synthesized voice, real-time stream) |
| Start, a short pause, then the dictation | component `.feature`, E2E `.feature` |
| Restart, a short pause, then the new dictation | component `.feature`, E2E `.feature` |
| A chain ending in start followed by the next dictation at once | component `.feature` |
| The user's recorded start start followed by the dictation at once | E2E `.feature` `@voice_samples` (the user's "start start" from `session-2`, a synthesized sentence right after it) |
| The user's recorded restart restart followed by the new dictation at once | E2E `.feature` `@voice_samples` (the user's "restart restart" from `session-5`, a synthesized sentence right after it) |
| No gap at the join (D2) | unit (pipeline: start, then heard sound, then join; cpal adapter join) |
| Opening "start" or "restart", also doubled, removed (D4) | unit (`command_words`; pipeline: a restarted dictation's text) |
| Manual run | the user: "start start" + speech, "start", pause, speech |

The E2E hands-free harness gets the two seams it lacks: its spotter hands out the adapter's heard sound, and its recorder puts the lead-in first. Its recorder also takes 0.3 s to open the microphone, as macOS does. The new scenarios stream the speech in real time, so the command is reported while the sentence is still being said, as with a real microphone. The silence `say` puts after a word is cut off, so a pause is as long as the scenario says. Without the fix, all five E2E scenarios lose the sentence's first words: "machine on the third floor is broken." after a pause, and "floor is broken." after "start, start".

## Risks / Trade-offs

- **Room sound at the start of a dictation.** A command said alone now brings its following pause into the recording. whisper-server's own voice activity detection skips silence, so this adds nothing to the text.
- **A dictation that really begins with "start".** Under D4, a sentence opening with "start" right after "start start" loses that word. This is the same trade-off `Head` already makes, and it is rare.
- **The join can repeat or drop up to one capture callback** (about 10 ms) of audio. That is too short to repeat or drop a word.
- **The user's quiet "restart" and a 0.4 s pause** run into one utterance with the speech after them (the voice activity detector does not end it), so a lone "restart" followed that soon by speech is no command. This is how utterances end today, not a part of this change; the user's-voice restart scenario uses the doubled "restart restart", which counts in the middle of speech. With a 0.4 s pause the service's check of a command can also take in the first sound of the next word ("Restart" + "c…" heard as "Restocked"), so the scenarios pause 0.5 s.
- **Drift between the sample clock and wall time** still affects the stop cut. It is out of scope, noted here, and not made worse.
