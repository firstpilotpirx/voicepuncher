# Design

## Context

See proposal.md § Why. The command spotting service (`command-spotting/src/engine.rs`) feeds every 32 ms window of the stream to the Silero voice activity detector (threshold 0.5, `VAD_SPEECH_THRESHOLD`); a confirmation by the speech recognition service is asked only for a speech segment the detector reports. The keyword spotter hears the same raw audio in 100 ms frames.

**Measured** (engine on the real models, `say`'s default voice, each clip scaled so its loudest 10 ms has the RMS named, 1 s of digital silence around it, no speech recognition service):

| level | today | detector hears it raised 3.5 dB (× 1.5) |
|---|---|---|
| 0.005, "clear all text" | no segment | one segment, 1.07–1.98 s |
| 0.005, the five other phrases and "clear" | one segment each | one segment each |
| 0.004, every phrase and short form | "clear all text" and "clear" get none | one segment each |
| 0.003, "clear all text", "start dictation", "cancel dictation" | no segment | "clear all text" none, the others one each |
| room noise alone, 20 s at RMS 0.003 / 0.01 / 0.03 | no segment | no segment |

A detector threshold of 0.4 also finds the 0.005 "clear all text", but merges the two recorded commands below and misses "clear all text" at 0.004.

**The upper bound, measured.** The user's session-1 recording has two "start dictation"s 0.32 s apart (labelled 4.98–6.19 s and 6.58–7.41 s), with a click of RMS 0.001 at 6.42 s in the room between them. Today and at × 1.5 and × 1.7 the detector ends the first at 6.26 s; at × 1.85 and × 2 it holds both in one segment (5.10–7.48 s), and the user's recorded commands test misses the second. × 1.5 is the lowest gain measured that notices every command at 0.004, below the user's quietest 0.0045, and stays clear of that merge.

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

### D1. The detector hears the stream raised 3.5 dB

`detect_speech` multiplies each window it feeds the detector by `DETECTOR_GAIN` = 1.5, clamped to ±1.0, in a pure helper `raised(window) -> Vec<f32>` next to it. Only the detector's copy is raised: the pause watch, the keyword spotter, the recent audio sent for confirmation and every reported time keep the raw stream. A fixed gain, not an adaptive one: an adaptive gain is at its highest in the silence before an utterance and so raises the onset just as much, while adding state.

*Alternatives:* × 2 (merges the user's two commands, see Context); threshold 0.4 (merges them too); raising the keyword spotter's input too (measured: at 2× and 4× a normal-level phrase after 25 s of silence is placed 0.36–0.4 s late, and at 4× one phrase is no longer spotted on its own).

### D2. Speech saved for tuning stays at the level heard

`save_speech` writes `segment.samples()`, which are now the raised copy; they are divided by `DETECTOR_GAIN` before saving, so a saved file has the level the microphone gave (a clipped sample comes back at 0.67, which no tuning relies on).

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
- **Integration** (`command-spotting/tests/engine_it.rs`, real models, no speech recognition service): the user's two recorded "start dictation"s 0.32 s apart give two segments (Red at × 2); each of the six phrases and each short form, scaled to a loudest 10 ms of 0.005, gives exactly one speech segment (Red: "clear all text" gives none); 20 s of room noise at RMS 0.003, 0.01 and 0.03 gives no segment (a guard: it passes before and after); the existing `should_spot_quiet_speech` keeps its lead and tail margins (it fails with D1 alone and passes with D3).
- **E2E** (`e2e-test/tests/features/voice_control.feature`): the `@wip` Examples row "clear all text" of "A command said in a quiet voice is recognized", added by `recognize-users-quiet-restart`, loses `@wip` and passes.

## Risks / Trade-offs

- [More segments in a quiet room] → each is one more confirmation request for a short utterance; room noise alone gives none (measured), and a confirmation turns into a command only when its transcript is a command phrase.
- [Earlier segment starts change the clip sent for confirmation] → the pre-roll already reaches 0.8 s before the start; `voice_forms_it`, `voice_samples_it` and the E2E seams and resilience suites in `just full-check` cover what is heard.

## Dependencies

Waits for `recognize-users-quiet-restart`: it adds the spec scenario "A command said in a quiet voice is recognized" this change's delta builds on and the `@wip` E2E row this change turns on, and it reworks the same confirmation path in `command-spotting/src/engine/`.

Waits for `fix-chained-start-after-cancel-enter`: `voice_forms_it` is red on `main` (the user's recorded "never mind cancel enter start" loses "start dictation", 4 out of 4 runs with `main`'s build and with this branch's), so `just check` and `just full-check` cannot pass here until that change lands. It is not in `orchestration.yaml`: kit accepts only a dependency that started before this change, and that one was filed after it; this branch merges `main` once that change is archived there.
