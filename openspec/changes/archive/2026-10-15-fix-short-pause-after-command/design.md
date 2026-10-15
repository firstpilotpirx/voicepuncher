# Design

## Context

The command spotting service (`command-spotting/src/engine.rs`) cuts the streamed audio into speech segments with the Silero voice activity detector of sherpa-onnx (`min_silence_duration` 0.3 s, speech threshold 0.5). One segment is one utterance. When a segment closes, the service sends the segment plus a 0.2 s post-roll (`POST_ROLL_SECS`) to whisper-server to confirm a command said on its own (`close_segment`).

Measurements on the real models (a scratch probe on the engine, the speech recognition service live) show where the pause goes:

| Audio | Pause | Silero segments |
|---|---|---|
| user's quiet "restart" (`session-2`, peak RMS ≈ 0.01) + digital silence + speech | 0.3 / 0.4 / 0.5 s | one segment, even with `min_silence_duration` 0.05 s |
| same | 0.6 s | split; the first segment ends 0.57 s after the word |
| same, room noise in the pause | 0.4 s | one segment; the first ends 0.16 s after the word |
| same, room noise in the pause | 0.5 s | split |
| `say` "Restart", trailing silence cut, + digital silence + "Coffee…" | 0.3 s | one segment |
| same | 0.4 s and longer | split; the segment ends 0.04 s after the word |

So Silero's probability stays above its end threshold for 0.15–0.55 s after a quiet word: the hangover eats the pause before `min_silence_duration` starts counting. A pause shorter than hangover + 0.3 s never ends the utterance. For louder synthesized speech the hangover is small, but the detector sees only about 0.26 s of a 0.3 s pause, short of the 0.3 s it needs.

The confirmation clip runs 0.2 s past a segment end. When the segment end lags the word (the hangover again), those 0.2 s reach into the next word: in a real-time E2E run "Restart", a short pause, "Coffee…" was heard as "Restocked". The probe with digital silence did not reproduce that case (its segment ends 0.04 s after the word); the user's quiet "restart" with 0.6 s does: its segment ends 0.57 s late, and the clip takes in the next sentence's first sound.

`say` ends every phrase with about 0.4 s of its own silence; the E2E step "takes a short breath" adds its 0.35 s after that, so it tests a 0.75 s pause.

## Goals / Non-Goals

**Goals:**

- A pause of 0.3 s of quiet after a word ends the utterance, for a quiet voice and for synthesized speech, whatever the detector's hangover.
- The confirmation clip of a segment never holds the next utterance's first sound.
- The E2E pauses are the pauses they name.

**Non-Goals:**

- A different voice activity or keyword spotting model.
- Changing the 0.3 s rule, the phrases, or the commands said without a pause.
- Splitting an utterance in a noisy room where the pause is not quieter than the speech; the detector alone decides there, as today.

## Decisions

### D1. A quiet run of 0.3 s ends the utterance, beside the detector

A new `PauseWatch` (`command-spotting/src/engine/pause.rs`) measures the loudness of every 10 ms frame of the stream while a segment runs. A frame is *quiet* when its RMS is at most `QUIET_FRACTION` (3 %) of the loudest frame of the running segment, or at most three times the room's noise floor (the quietest frame of the last 3 s), capped at 30 % of the loudest frame. 3 % keeps the "s" and "t" that end a word in a loud voice (5–14 % of its vowels) inside the utterance; the noise floor term lets a quiet voice's pause, which is room noise, count as quiet. A pause of 0.3 s holds two fewer whole 10 ms frames, the ones it shares with the words around it, so 28 quiet frames end the utterance. When quiet frames have run that long and the detector still holds the segment, the session flushes the detector (`VoiceActivityDetector::flush`), which closes the segment at once, and ends it where the quiet run began. The detector is then reset, as after every segment today.

The level is relative to the segment's own speech, so the user's quiet voice (peak ≈ 0.01) and a loud `say` voice are treated alike. A segment whose pause is not that quiet is left to the detector, as today.

Alternatives:

- **Lower `min_silence_duration`.** Rejected: with digital silence after the user's quiet word, even 0.05 s did not split a 0.5 s pause; and a lower value splits louder speech at pauses shorter than 0.3 s.
- **Raise the speech threshold**, so the detector ends sooner. Rejected: it misses quiet speech starts, the user's main voice.
- **Run Silero ourselves to read its probabilities.** Rejected: the sherpa-onnx API gives no per-window probability, and a second model run per window costs CPU for no better signal than loudness on a pause.

### D2. The confirmation clip stops at the next sound

`close_segment` sends `from..to` with `to = min(end + POST_ROLL, next sound)`. The *next sound* is the first 10 ms frame after the segment end that is louder than the quiet level of D1, searched in the audio already heard; with none, `to` stays `end + POST_ROLL`. A segment ended by D1 always has 0.3 s of quiet after its end, so its post-roll is whole. A segment ended late by the detector loses only the part of its post-roll that is the next word.

### D2b. A phrase spotted after a quiet pause closed its segment goes before the segment's end

The keyword spotter reports a phrase about 0.25 s after its end, plus decoding time. When the detector closed segments late, the phrase was out before the segment's end; a segment closed by D1 may end first. Such a phrase belongs to the segment that ended last (the orderer already sends it at once). While that segment's events are held for its confirmation, `Outbox::send_in_last_segment` puts the phrase before the held `SpeechEnd`, so the client still sees start, phrase, end. A phrase the spotter reports after its segment's events went out follows the `SpeechEnd`; the app's recognizer already takes a phrase of the segment that closed last (its confirmations always come that way), so `commandspotting_websocket_it` checks that the phrase is reported, not that it comes before the end. Holding a segment until the spotter catches up was tried and dropped: the client waits for a sync before it sends more audio, so the hold stalled it.

### D2c. The speech recognition's command outweighs the keyword spotter's

When the confirmation of a short utterance hears a command, the keyword spotter's phrases of other commands held in that segment are dropped (`Outbox::keep_only_phrase`). Before, the app's recognizer took the utterance as mixed and carried out nothing. The user's "restart dictation please" (heard "restore dictation please", which the matcher accepts) was spotted as "start dictation"; it worked only because the detector's late close left that phrase without a segment, so it was confirmed on its own.

### D2d. The tail of an utterance a quiet pause closed is heard again

The detector's late close also left time for one more window search over the end of an utterance. Without it, the user's "we meet on Friday at ten enter" is heard whole as "at 10am", and "enter" only in a clip of its last 1.5 s. So when a quiet pause (D1) closes an utterance longer than 2 s and its confirmation holds no command, its last 1.5 s are heard again (`JobKind::Tail`, `TAIL_CHECK_SECS`) and the segment's events wait for that answer (`Outbox::transfer`). The user chose this over dropping the recording from the tests: one more request to the speech recognition service per such utterance.

### D3. E2E pauses are cut from sound to sound

The steps that build a pause from `say` speech cut the speech's own leading and trailing silence first (`without_trailing_silence` and a new `from_first_sound` in `e2e-test/src/voice_commands.rs`, shared by every step; `without_lead_pause` moves there too and keeps cutting only the padding, for the scenarios with no pause). "Takes a short breath" then pauses 0.35 s of real silence. The new steps do the same.

### D4. Tests on the user's recordings cut each command with the labelled margins

The user's recorded "restart", "start", "stop" and "enter" said alone come from their labelled sessions (`forms.jsonl`), with the lead and tail margins of `recorded_form` (0.3 s and 0.2 s), cut after their last 10 ms louder than 3 % of their loudest (`until_last_sound`: the room's silence after the word would lengthen the pause), followed by 0.3, 0.45 and 0.6 s of silence and a sentence. The "restart" at 27.23 s in `session-2` is left out (`UNSTEADY_RECORDINGS`): whisper hears it as "Let's start" whatever the pause, which is the backlog change `recognize-users-quiet-restart`. Each runs through the real service in the state that accepts the command. The existing labels cover all four forms, so no new recording session is needed.

## Test levels

| Spec scenario | Level |
|---|---|
| Stop after a short breath (now a real 0.35 s) | E2E `.feature` (`@smoke @serial`) in the hands-free session, checking the paste; the step builds the breath per D3. `placed_commands_it` checks the breath gives two utterances, its breath also cut from sound to sound |
| Restart, a short pause, then speech that starts like another word | E2E `.feature` (`@smoke`) in real time; engine integration test (`engine_it.rs`) for the clip end |
| A command alone, a short pause, then speech | E2E `.feature` (`@smoke`); engine integration test for the split |
| The user's recorded command alone, a short pause, then speech | E2E `.feature` (`@smoke @voice_samples`) |
| (D1, D2, D2b–D2d internals) | unit tests in `engine/pause.rs` and `outbox.rs`; engine integration test for the tail check (D2d); `voice_forms_it` on the user's recordings; `commandspotting_websocket_it` checks the phrase is reported with its speech |

Existing negatives keep guarding the "no other command" side: the corpus speech scenarios (`@accuracy`), short Russian phrases and the command words inside a sentence.

## Risks / Trade-offs

- **More segments in dictation speech.** A sentence with a 0.3 s quiet pause now splits there, as the spec always said. More short segments mean more confirmation requests; each is short. The corpus "no command in ordinary speech" scenarios check that no command comes of it.
- **A word that fades slowly** may be cut a few frames early by D1; the pre-roll and the 0.2 s post-roll cover it.
