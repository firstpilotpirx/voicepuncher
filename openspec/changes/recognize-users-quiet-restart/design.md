# Design

## Context

See proposal.md § Why. `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt` are on `main` (v0.14.3).

**How a short utterance is confirmed.** When a segment of at most 3 s (`max_command_secs`) closes — by the voice activity detector, or by a quiet run of 0.3 s — `close_segment` (`command-spotting/src/engine/confirming.rs`) sends the audio from 0.8 s before the segment start (`PRE_ROLL_SECS`, never before the end of the previous utterance) to the segment end, run on until the post-roll is over or the next sound starts (`first_sound`). The request (`command-spotting/src/confirm.rs`) asks for English, temperature 0, no retry (`temperature_inc = 0`) and word times, with no prompt. `apply_segment` takes the whole transcript as a command phrase or short form (`PhraseMatcher::phrase_in`) and places the event by the first word's time, counted from the clip start. A segment's events are held in the `Outbox` until its confirmation is answered; a quiet-pause segment that heard no command already moves its hold to a second job (the tail check, `Outbox::transfer`).

**What else decides what is heard.**

- whisper-server runs its own voice activity detector on every request (`--vad -vsd 2000 -vp 500`) and decodes the speech it finds, extended by 0.5 s on each side.
- The keyword spotter knows only the full phrases (`command-spotting/keywords.txt` has "restart dictation", never "restart"): a lone "restart" counts only when its confirmation hears it.

**The recording** (`~/.voicepuncher/voice-samples/session-2.wav`, 20 ms RMS):

| stretch | level |
|---|---|
| room before, up to 26.66 s | 0.0002–0.0005 |
| faint sound, 26.70–27.18 s | 0.0009–0.0026 |
| "re", 27.20–27.25 s | 0.0033–0.0049 |
| a gap, 27.26–27.30 s | 0.0004–0.0008 |
| "start", 27.32–27.52 s | 0.0022–0.0092 |

The two other recorded "restart"s of the session (28.50 s, 29.68 s) have room level up to the word.

## Goals / Non-Goals

**Goals:**

- The user's restart at 27.23 s is heard as "restart" during a dictation after every pause from 0.3 to 0.8 s, alone and with speech after it.
- No other confirmation sends a different clip: what is heard for every other recorded or synthesized utterance stays as it is.
- No new way for "start" or "let's start" to become a restart.

**Non-Goals:**

- Near misses of other commands; the clips of a running utterance's checks (head, windows, tail); the clip of a phrase no segment took.
- Where a segment ends and where its clip ends; the request fields (prompt, retries); the keyword list, the models, whisper-server's flags.

## Decisions

### D1. Measured with the three changes on `main`

The clip of the 27.23 s restart, sent with the confirmation request to a speech recognition service of its own (port 9281, three copies), with *a* seconds of the recording before the label and *s* seconds of digital silence in front (§ Results). Then every short labelled form of sessions 2–5 (72 forms of at most 2.5 s), today's clip against each steady clip variant; then "start", "let's start" and "restart" synthesized by three `say` voices, at `say`'s level and scaled to the quiet level, through today's clip and through the steady clip. The scripts stay outside the repository; the tables are in § Results.

**Exit taken.** No steady clip variant keeps every recorded form as it is heard today (D2 is rejected as the fix), but the steady clip with gain hears the restart as "Restart." after every silence, and never turns a synthesized "start" or "let's start" into "Restart.": D3, the second hearing, is the fix, and the steady clip is its clip.

### D2. Rejected as the fix: a steady clip for every short utterance

A fixed lead before each short utterance's first sound, raised toward a normal level, in place of today's pre-roll clip. With an onset share of 0.4, leads of 0.2, 0.25 and 0.3 s, with or without gain, 23 to 30 of the 72 recorded forms are heard differently, and each variant breaks some: lead 0.25 s turns "restart dictation" (54.34 s) into "That is tar dictation." and "cancel restart" (session-5 30.11 s) into "Cancelery start."; lead 0.3 s turns "start please" (36.02 s) into "Start the please."; lead 0.2 s turns "restart restart" (session-5 27.80 s) into "start to start.". A clip that changes what is heard for a third of the user's commands to fix one is not steady in the sense that matters.

### D3. The fix: a second hearing of a near miss of "restart"

- **Trigger.** The answer to the segment confirmation of a short utterance makes no command (`apply_segment` gives none), "restart dictation" is among the selected phrases (a dictation runs), and the transcript is at most three words (by the matcher's words: "Let's start." is "let", "s", "start") whose last word is a listed near miss of the one-word form of a selected command. The list, `NEAR_MISSES` next to `ALIASES` in `matcher.rs`, holds one entry: "start" for "restart dictation". A list, not an edit distance, so "cancelled" never asks for a cancel. `PhraseMatcher::near_miss(transcript) -> Option<String>` gives the command the transcript nearly says.
- **The steady clip** (`command-spotting/src/engine/clip.rs`, a pure function): from the segment's confirmation audio (pre-roll start to clip end) it finds the first sound — the first 10 ms window, slid by 1 ms, louder than the onset level, the larger of the pause watch's quiet level and 0.4 of the clip's loudest 10 ms window (`ONSET_SHARE`) — and starts the clip 0.25 s before it (`LEAD_SECS`), padded with digital silence where the confirmation audio starts later. A clip whose loudest window is below 0.1 (`TARGET_LEVEL`) is raised toward it, by at most 10 times (`MAX_GAIN`), never lowered. It gives the samples and the origin: the stream position its first sample stands for, padding included. At 0.4 the faint sound before the user's "re" (at most 0.0026 against a loudest window of about 0.01) lies under the onset and the "re" above it, so the first sound is the "re"; a lead from 0.2 to 0.3 s hears the restart right after every silence (§ Results), and 0.25 s sits between.
- **Asking.** `apply` sees the near miss and asks a `Rehear` job with the steady clip; the outbox moves the segment's hold to it, as the tail check does. `ask` takes the audio to send instead of cutting it itself, so the steady clip goes out through it.
- **Counts** only when the second transcript as a whole is the near-missed command (`phrase_in` gives "restart dictation"); the event is a `Phrase` placed by the second clip's first word and origin, as `apply_segment` places one. Any other answer, or a second hearing given up, is no command.
- **Not asked:** for a long utterance ("we are all here, let's start" in one breath), for a transcript of more than three words ("let's start with the budget"), outside a dictation (no "restart dictation" selected), for an unheld phrase, a head, window or tail check.
- **Cost.** One more confirmation, only for a near miss: about 0.15 s on an idle service (p95 of the user's command clips without retry, `limit-confirmation-fallback`), within the confirmation deadline.
- **Logs:** the second hearing logs its lead, gain and answer at debug level next to its job line.

**Alternatives:**

- *Take "let's start" or "start" as "restart" during a dictation.* Rejected (proposal): a wrong restart throws the dictated text away.
- *The steady clip for every short utterance* (D2). Rejected: it changes what is heard for a third of the user's commands and breaks some.
- *The prompt for confirmations.* Rejected by `restore-recognition-prompt`'s measurement: false commands elsewhere.
- *The second hearing with today's clip and another pre-roll.* Rejected: with the faint sound in the clip, every lead longer than 0.4 s gives "Let's start.", and without gain, leads of 0.2–0.4 s flip between "Restart.", "Start" and "Let's start." with the silence in front (§ Results).
- *A second hearing for every short utterance that heard no command.* Rejected: one more request for every word said alone during a dictation, and a second chance for anything to become a command.

### D4. The exclusion and the known-wrong entry go

`UNSTEADY_RECORDINGS`, the filter in `recorded_form` that uses it and their comments are removed: `recorded_form("restart")` gives the restart at 27.23 s, so the restart rows of "The user's recorded command alone, a short pause, then speech" run on it. The `KNOWN_WRONG` entry for session-2 27.23 s in `app/tests/voice_forms_it.rs` is removed (the list stays, empty, for the next known-wrong form). Both removals are the reproduction: they turn those tests red until D3 lands.

## Test levels

| Spec scenario (voice-control) | Test level |
|---|---|
| Every recorded command of the user is recognized | adapter integration, unchanged: `voice_samples_it`, `voice_forms_it` with no known-wrong entry (the 27.23 s restart is checked again), in `just e2e-smoke` |
| A short Russian phrase is no command | E2E `.feature`, the existing `@smoke` scenario |
| The user's quiet restart after any pause before it | E2E `.feature` (`@smoke @voice_samples`) Scenario Outline: a dictated sentence, 0.3, 0.4 … 0.8 s of silence, the recorded restart, while dictating: "restart dictation" once. Engine integration in `engine_it.rs` with a scripted service: a short utterance heard "Let's start." then "Restart." gives one restart; unit tests of `engine/clip.rs` and of `near_miss` |
| The user's quiet restart, a short pause, then speech | E2E `.feature`: the restart rows of `fix-short-pause-after-command`'s outline run on the recording (D4) |
| A command said in a quiet voice is recognized | E2E `.feature` (`@smoke`) Scenario Outline: the six command phrases and six short forms at the quiet level, each in the state that accepts it; the user's own quiet commands are in `voice_forms_it` |
| Stop dictation without a dictation is ignored | component and E2E `.feature`, existing, unchanged |
| Clear during a dictation is ignored | component `.feature`, existing, unchanged |
| Start or let's start on its own during a dictation is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "start" and "let's start" after a sentence, at `say`'s level and at the quiet level, while dictating: no command. Engine integration: "Let's start." heard twice gives no command. Unit: `near_miss` and `phrase_in` take "Let's start." as no command |
| Start ending a dictated sentence is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "we are all here, let's start" and "the meeting is about to start", at both levels. Engine integration: a long utterance ending in "let's start" asks no second hearing |
| Start inside a dictated sentence is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "let's start with the budget" and "we start at nine tomorrow", at both levels. Unit: `near_miss` takes neither |

- The new E2E scenarios sit under the rules of their requirements in `e2e-test/tests/features/voice_control.feature` and stream at once through the real command spotting service, as "A command word inside a sentence is still no command" does (`carried_out_by_app` with the dictating phrases): no command there means the dictation was never left. The restart's steps pick the recording by its label (session and start time), not as the first match.
- **Quiet level:** `say` speech scaled so its loudest 10 ms window has an RMS of 0.005, the level of the user's quietest labelled commands (0.0045 at the lowest); a helper in `e2e-test/src/voice_commands.rs`.
- **Unit tests of `engine/clip.rs`:** the first sound is found at the same place in the audio wherever the audio starts; the lead is cut to its length, and padded with silence when the audio starts later; nothing louder than the onset level is left out; the gain applies only below the target, never above the cap, never below 1; the origin counts the padding.
- **Ports:** the E2E scenarios and the voice-sample tests reach the command spotting service on its port, and the recipes reuse a service already running on 8082, built from `main`. `just check` and `just full-check` run on ports of their own (`VOICEPUNCHER_STT_PORT=9281 VOICEPUNCHER_LLM_PORT=9280 VOICEPUNCHER_KWS_PORT=9282`).

## Risks / Trade-offs

- [A quiet "let's start" or "start" said during a dictation is heard "Restart." the second time and throws the dictation away] → the second hearing must say "restart dictation" or "restart" as a whole; the measurement heard "start" and "let's start" of three voices at both levels as themselves in it; the E2E negatives run with it.
- [A raised clip raises its noise] → only the second clip of a near miss is raised, by at most 10 times; the room at 0.0002–0.0005 stays below 0.005.
- [The second hearing delays the near miss's events by one request] → about 0.15 s, only for a near miss during a dictation.
- [The near-miss list grows into a second matcher] → one entry, the one the measurement shows; another needs its own recording and change.
- [Already seen and out of scope: `say`'s Daniel voice saying "start" at the quiet level is heard as "Stop." by today's clip, which would end a dictation] → the E2E negatives use the default voice, as every other E2E scenario does; left as a finding for its own change.

## Migration Plan

None: no setting, protocol or data change; the steady clip's shape is constants in the code. Rolling back is reverting the change's commits, the tests and the removed exclusion with them.

## Results

Measured on 2026-10-17, `ggml-small.bin`, whisper-server 1.9.5 with the justfile flags, three copies behind HAProxy on port 9281 (the shared services were not used); confirmation request as `confirm.rs` sends it.

### The 27.23 s restart: *a* s before the label, *s* s of digital silence in front

| a \ s | 0 | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 | 0.6 | 0.8 |
|---|---|---|---|---|---|---|---|---|
| 0–0.1 | Start. | Start. | Start. | Start. | Start. | Start. | Start. | Start. |
| 0.2 | Restart. | Restart. | Restart. | Restart. | Start | Start | Start | Restart. |
| 0.3 | Restart. | Restart. | Let's start. | Restart. | Let's start. | Let's start. | Restart. | Restart. |
| 0.4 | Let's start. | Let's start. | Restart. | Let's start. | Let's start. | Start | Start | Let's start. |
| 0.5–0.8 | Let's start. | Let's start. | Let's start. | Let's start. | Let's start. | Let's start. | Let's start. | Let's start. |

Raised 10 times (loudest window 0.0092 → 0.092), *s* = 0, 0.2, 0.3, 0.4, 0.5, 0.6, 0.8:

| a | heard |
|---|---|
| 0.1 | Start. (every cell) |
| 0.2 | Restart. (every cell) |
| 0.3 | Restart. (every cell) |
| 0.4 | Let's start. at s 0; Restart. at 0.2–0.6; Start at 0.8 |
| 0.5–0.6 | Let's start. at s 0 and 0.8 (0.6 at 0.6 too); Restart. otherwise |

Raised 3 times: at a 0.2 and 0.3 one cell each still gives "Start." or "Let's start.".

### Every short recorded form, today's clip against the steady clip (onset share 0.4)

| variant | forms heard differently (of 72) | broken |
|---|---|---|
| lead 0.25 s, gain to 0.1 (cap 10) | 30 | 27.23 s fixed; "restart dictation" 54.34 s → "That is tar dictation."; session-5 "restart restart" → "restart and start", "cancel restart" → "Cancelery start." |
| lead 0.25 s, no gain | 30 | 27.23 s fixed; the same three broken |
| lead 0.3 s, no gain | 27 | 27.23 s fixed; "start please" 36.02 s → "Start the please." |
| lead 0.2 s, no gain | 23 | 27.23 s fixed; session-5 "restart restart" → "start to start." |

The other differences are punctuation and case. Today's clip hears no recorded form as a near miss of "restart" but the 27.23 s restart.

### Near misses through the second hearing (lead 0.25 s, gain to 0.1, cap 10)

"start", "let's start", "restart" (with and without a full stop) by `say`'s default, Daniel and Samantha voices, at `say`'s level and at the quiet level (loudest window 0.005), after 0.8 s of silence: the second hearing gives what was said every time — "Start.", "Let's start.", "Restart." — never "Restart." for "start" or "let's start". Today's clip hears Daniel's quiet "start" as "Stop." (Risks).
