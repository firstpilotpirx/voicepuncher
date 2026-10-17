# Design

## Context

See proposal.md § Why. The command spotting service (`command-spotting/src/engine.rs`) feeds every 32 ms window of the stream to the Silero voice activity detector (threshold 0.5, `VAD_SPEECH_THRESHOLD`); a confirmation by the speech recognition service is asked only for a speech segment the detector reports. The keyword spotter hears the same raw audio in 100 ms frames.

**Measured** (engine on the real models, `say`'s default voice, each clip scaled so its loudest 10 ms has the RMS named, 1 s of digital silence around it, no speech recognition service):

| level | today | detector hears it raised 6 dB |
|---|---|---|
| 0.005, "clear all text" | no segment | one segment, 1.07–1.98 s |
| 0.005, the five other phrases and "clear" | one segment each | one segment each |
| 0.003, "clear all text", "start dictation", "cancel dictation" | no segment | one segment each |
| room noise alone, 20 s at RMS 0.003 / 0.01 / 0.03 | no segment | no segment |

A detector threshold of 0.4 also finds the 0.005 "clear all text", but misses it again at 0.003: a 6 dB raise leaves more margin below the user's quietest commands (0.0045).

**A side effect, measured.** Heard raised, the detector notices a quiet "start dictation" (`say` × 0.1) at 1.034 s instead of 1.13 s; its speech starts at 1.01 s. The keyword spotter reports that phrase's tokens at 1.32, 1.64, 1.72, 1.80, 1.92 s; the gap of 0.32 s after the first token is over `MAX_TOKEN_GAP_SECS` (0.3 s), so `without_stray_lead` drops the "START" token as one heard in noise and places the phrase at 1.42 s — 0.39 s after its segment starts, past the app's 0.3 s lead margin (`LEAD_MARGIN`), so without a speech recognition service it would no longer count as said on its own. At a normal level the gap is 0.28 s. Today the late detector hides this.

## Goals / Non-Goals

**Goals:**

- A quiet "clear all text" said on its own gets its speech segment, its confirmation, and takes effect.
- Every command still gets one segment at every level it gets one today; room noise alone still gets none.
- A quiet phrase spotted by keyword alone is still placed within its segment.

**Non-Goals:**

- A different detector, keyword spotting or speech recognition model; new phrases.
- The level the keyword spotter or the speech recognition service hears (both keep the raw audio).

## Decisions

### D1. The detector hears the stream raised 6 dB

`detect_speech` multiplies each window it feeds the detector by `DETECTOR_GAIN` = 2.0, clamped to ±1.0, in a pure helper `raised(window) -> Vec<f32>` next to it. Only the detector's copy is raised: the pause watch, the keyword spotter, the recent audio sent for confirmation and every reported time keep the raw stream. A fixed gain, not an adaptive one: an adaptive gain is at its highest in the silence before an utterance and so raises the onset just as much, while adding state.

*Alternatives:* threshold 0.4 (misses the 0.003 clip, see Context); raising the keyword spotter's input too (measured: at 2× and 4× a normal-level phrase after 25 s of silence is placed 0.36–0.4 s late, and at 4× one phrase is no longer spotted on its own).

### D2. Speech saved for tuning stays at the level heard

`save_speech` writes `segment.samples()`, which are now the raised copy; they are divided by `DETECTOR_GAIN` before saving, so a saved file has the level the microphone gave (a clipped sample comes back at 0.5, which no tuning relies on).

### D3. A first token up to 0.4 s before the next is no stray

`MAX_TOKEN_GAP_SECS` goes from 0.3 to 0.4 s. The stray token the rule is for fires about half a second before the rest (the unit test's 0.48 s); a quiet "start" before "dictation" is 0.32 s. The rule's unit test keeps its 0.48 s stray; a new one keeps a 0.32 s first token.

### D4. The quiet "start" heard as "Stop."

The proposal asks whether `say`'s Daniel voice saying "start" at the quiet level, heard as "Stop." by the confirmation, needs a fix here. `recognize-users-quiet-restart` owns the rows that cover it ("start" and "let's start" on their own and in sentences, in a quiet voice, while dictating: no command takes effect), and this change waits for it (§ Dependencies). This change adds no fix for it; its acceptance bar is that those rows still pass with the detector raised. If one fails because of D1, that is a `scope` question to the user, not a silent loosening.

## Test levels

| Scenario (voice-control) | Level |
|---|---|
| A quiet clear all text clears the text | E2E (the quiet-command outline's "clear all text" row) and integration (`engine_it.rs`, one segment per quiet phrase) |
| A command said in a quiet voice is recognized | E2E, unchanged rows; run again with the detector raised |
| Every recorded command of the user is recognized | integration (`app/tests/voice_forms_it.rs`, `voice_samples_it.rs`), unchanged; run in `just check` |
| A short Russian phrase is no command | E2E, unchanged; run in `just check` |
| The user's quiet restart after any pause before it | E2E, unchanged (from `recognize-users-quiet-restart`) |
| The user's quiet restart, a short pause, then speech | E2E, unchanged (from `recognize-users-quiet-restart`) |

- **Unit** (`command-spotting/src/engine.rs`): `raised` doubles a window and clamps at ±1.0; `phrase_span` keeps a first token 0.32 s before the next and still drops one 0.48 s before.
- **Integration** (`command-spotting/tests/engine_it.rs`, real models, no speech recognition service): each of the six phrases and each short form, scaled to a loudest 10 ms of 0.005, gives exactly one speech segment (Red: "clear all text" gives none); 20 s of room noise at RMS 0.003, 0.01 and 0.03 gives no segment (a guard: it passes before and after); the existing `should_spot_quiet_speech` keeps its lead and tail margins (it fails with D1 alone and passes with D3).
- **E2E** (`e2e-test/tests/features/voice_control.feature`): the `@wip` Examples row "clear all text" of "A command said in a quiet voice is recognized", added by `recognize-users-quiet-restart`, loses `@wip` and passes.

## Risks / Trade-offs

- [More segments in a quiet room] → each is one more confirmation request for a short utterance; room noise alone gives none (measured), and a confirmation turns into a command only when its transcript is a command phrase.
- [Earlier segment starts change the clip sent for confirmation] → the pre-roll already reaches 0.8 s before the start; `voice_forms_it`, `voice_samples_it` and the E2E seams and resilience suites in `just full-check` cover what is heard.

## Dependencies

Waits for `recognize-users-quiet-restart`: it adds the spec scenario "A command said in a quiet voice is recognized" this change's delta builds on and the `@wip` E2E row this change turns on, and it reworks the same confirmation path in `command-spotting/src/engine/`.
