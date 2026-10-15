# Design

## Context

See proposal.md § Why. This change lands after `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt`; what follows describes `main` with all three in place, as their branches and designs show them today.

**How a short utterance is confirmed.** When a segment of at most 3 s (`max_command_secs`) closes — by the voice activity detector, or by a quiet run of 0.3 s after `fix-short-pause-after-command` — `close_segment` (`command-spotting/src/engine/confirming.rs`) sends the audio from 0.8 s before the segment start (`PRE_ROLL_SECS`, never before the end of the previous utterance) to the segment end plus 0.2 s, stopped at the next sound. The request asks for English, the command phrases as a prompt, temperature 0 and word times; `limit-confirmation-fallback` (in review) adds `temperature_inc = 0`, so it is decoded once with no retry, and `restore-recognition-prompt` makes the prompt take effect if its measurements keep it for confirmations. `apply_segment` takes the whole transcript as a command phrase or short form (`PhraseMatcher::phrase_in`) and places the event by the first word's time, counted from the clip start. The silence before the command in that clip is whatever the pre-roll catches: 0.8 s minus the detector's delay, cut short by the previous utterance.

**What else decides what is heard.**

- whisper-server runs its own voice activity detector on every request (`--vad -vsd 2000 -vp 500`). It decodes only the speech that detector finds, extended by 0.5 s on each side. Of a lead longer than 0.5 s the decoder hears only the 0.5 s before the onset that detector finds, and a faint onset found late moves that cut.
- The keyword spotter knows only the full phrases (`command-spotting/keywords.txt` has "restart dictation", never "restart"). A lone "restart" gives no spotted phrase: it counts only when its confirmation hears it, and a confirmation that hears "Let's start." leaves nothing that counts.

**The recording, measured for this design** (`~/.voicepuncher/voice-samples/session-2.wav`):

| "restart" at | loudest 10 ms window (RMS) | before the word | its "re" against its "start" (50 ms windows) |
|---|---|---|---|
| 27.23 s (heard two ways) | 0.0100 | a faint sound for about 0.5 s right up to the "re": 0.0010–0.0022, against 0.0002–0.0005 before it | 0.0046 against 0.0074 |
| 28.50 s | 0.0088 | room level (0.0002–0.0006) up to the word | 0.0069 against 0.0072 |
| 29.68 s | 0.0111 | room level up to the word | 0.0079 against 0.0098 |

So the failing restart is not quieter than the two that work. The user's short labelled commands (sessions 2–5) peak between 0.0045 and 0.098, median 0.013; `say` speech peaks at about 0.24. What sets this one apart is the faint sound and the weak "re" in front of its "start", which some lengths of silence before them turn into "Let's". The E2E cut (`recorded_form`: 0.3 s before the label) keeps the last 0.25 s of that faint sound; the user's stream has all of it. The first measurement ("Restart." after 0.3, 0.6 and 0.8 s, "Let's start." after 0.4 and 0.5 s) was taken without the prompt and with whisper's default retries (temperature step 0.2), so one run could differ from the next ("Start." in some E2E runs).

**Tests that exist.** `command-spotting/tests/engine_it.rs` runs the real spotter and detector without a speech recognition service (`just test`); `placed_commands_it.rs` adds the real speech recognition service (`just e2e-smoke`); `app/tests/voice_samples_it.rs` and `voice_forms_it.rs` play the user's recordings through the command spotting service — `voice_forms_it` plays the 27.23 s restart alone, after 1 s of silence. The E2E recipes, `just check` and `just full-check` reuse a command spotting service already running on port 8082, which may be an older build than the branch.

**Re-read once the three changes are on `main`** (names and values may differ from their branches today):

- `fix-short-pause-after-command`: `close_segment` and its quiet level argument, the clip end (`first_sound`), `PauseWatch::quiet_level` and the loudness helpers in `command-spotting/src/engine/pause.rs`, the held segment in `Outbox` (`hold` with the position heard until); in the E2E, `recorded_form` with `LEAD_MARGIN_SECS`/`TAIL_MARGIN_SECS`, `UNSTEADY_RECORDINGS`, `command_pause_sentence`, the step "the user's recorded … said on its own, is followed by … s of silence and a sentence while …", and the helpers moved to `e2e-test/src/voice_commands.rs` (`from_first_sound`, `without_trailing_silence`); in `engine_it.rs`, `users_quiet_restart`, `KeepsClips`, `holds`, `sound_only`.
- `limit-confirmation-fallback`: the retry step of the confirmation request (`TEMPERATURE_STEP` in `command-spotting/src/confirm.rs`, 0 on its branch).
- `restore-recognition-prompt`: the confirmation request fields (`carry_initial_prompt`, `max_context`) and its § Results — whether confirmations kept the prompt.

## Goals / Non-Goals

**Goals:**

- The clip that confirms a short utterance depends on the utterance, not on the pause before it: the same words get the same answer.
- The user's restart at 27.23 s is heard as "restart" after every pause from 0.3 to 0.8 s, alone and with speech after it.
- No new way for "start" or "let's start" to become a restart.

**Non-Goals:**

- The clips of a running utterance's checks (head, windows), the last 2.5 s of a long utterance, and a phrase no segment took: only a short utterance's clip has a silence before the command that varies with the pause.
- Where a segment ends and where its clip ends (`fix-short-pause-after-command`); the request fields (prompt, retries).
- The keyword list, the models, whisper-server's flags (shared with every dictation request).

## Decisions

### D1. Measure first, with the three changes on `main`

**What.** Through the real confirmation path: an engine session in the test process — the branch's keyword spotter, detector, pause watch and `close_segment` — confirming with the real speech recognition service (port 8081, three copies behind HAProxy). Each run records the clip sent (its lead before the first sound, its loudest window), the transcript, the phrases the keyword spotter reported and the events.

| Series | Stream | Cells |
|---|---|---|
| quiet restart alone | a dictated sentence (`say`, trailing silence cut), P s of silence, the 27.23 s restart cut as the E2E cuts it, 1.5 s of silence; dictating | P = 0.3, 0.4, 0.5, 0.6, 0.7, 0.8 |
| quiet restart, speech after | the same up to the restart, then S s of silence and a sentence | P = 0.3, 0.5, 0.8 × S = 0.3, 0.45, 0.6 |
| whole faint sound | as "alone", the restart cut 0.6 s before its label, as the session has it | P = 0.3 … 0.8 |
| room in the pause | as "alone", the pause filled with the user's room (`session-1.wav` 0.5–4.5 s, as the E2E steps use) | P = 0.3 … 0.8 |
| the other restarts | the restarts at 28.50 s and 29.68 s, as "alone" | P = 0.3 … 0.8 |
| quiet commands | each command phrase and short form, `say` at the quiet level (§ Test levels), alone after 1 s of silence, in the state that accepts it | 12 |
| negatives | "start", "let's start" alone after a sentence; "we are all here, let's start", "the meeting is about to start"; "let's start with the budget", "we start at nine tomorrow" — at `say`'s level and at the quiet level; dictating | 12 |

Every cell runs 10 times. With no retry a clip should give one answer, but the three copies on Metal and the clip itself may still vary, and the first measurement saw "Start." in some runs.

**Tool.** A measurement, not a gate: `#[ignore]`d tests in a new `command-spotting/tests/confirmation_sweep_it.rs` that print one table per series, gated like `placed_commands_it` on the model paths, `VOICEPUNCHER_TRANSCRIPTION_URL` and `VOICEPUNCHER_VOICE_SAMPLES`. Running in process, they measure the branch's engine even while the shared command spotting service on 8082 runs an older build. The speech recognition service on 8081 is shared, so they run under the machine lock: start it with `just start-model-servers whisper-server`, then `~/.claude/bin/with-machine-lock cargo test -p voicepuncher-command-spotting --test confirmation_sweep_it -- --ignored --nocapture`, with the variables the justfile exports. They stay in the repo, to measure again when a model, the prompt or the clip changes. The tables go into § Results.

**Exit.**

- Every cell right 10 times out of 10 — each restart and quiet command gives exactly its command, each negative none: D2 and D3 are not built. The change removes the exclusion (D4) and adds the tests of § Test levels that do not rest on D2, and the proposal's Release section is changed to "no release": tests only, the version stays.
- A quiet command the detector does not notice at all (no segment, so no confirmation) is beyond what the clip can fix: stop and ask the user, it is outside the proposal's scope.
- Otherwise D2, and D3 only if no variant of D2 is right in every cell.

### D2. A steady clip for the confirmation of a short utterance

For the segment confirmation of a short utterance only, the clip is cut by the utterance's own sound, not by where the detector noticed it:

- **First sound:** the first 10 ms window, slid sample by sample from the pre-roll start, that is louder than the onset level — the larger of the pause watch's quiet level and a share of the utterance's loudest window. Sliding by sample places it by the audio, whatever frames the stream's messages fall on.
- **Lead:** the clip starts a fixed lead before the first sound: the stream's own audio as far back as the end of the previous utterance, digital silence in front of that. Only audio quieter than the onset level is ever left out, so no speech louder than it is cut.
- **Level**, only if the measurement needs it: a clip whose loudest window is below a target level is raised toward it, by at most a gain cap. A louder clip is never touched, so the confirmations of a normal voice keep their level.
- **End:** unchanged — the end `fix-short-pause-after-command` gives it.
- **Times:** the clip's origin, the stream position its first sample stands for (padding included), takes the place of the pre-roll start in `SegmentSpan`, so word times map back into the stream as today. The gain moves no time.

**Values, from the measurement.** The variants run D1's series through the engine: onset share 0 (the quiet level alone: the faint sound is the first sound), 0.2 and 0.4 (the "re" is the first sound, the faint sound lies in the lead); lead 0.2, 0.3, 0.4 and 0.5 s; gain off, or toward a loudest window of 0.03 or 0.1 with a cap of 10. A variant is dropped at its first wrong answer. The pick: right in every cell 10 times out of 10, negatives included; gain off before gain on; then a lead whose neighbours in the sweep pass too, so a first sound found a few milliseconds off does not flip the answer. The lead stays at most 0.5 s: whisper-server's detector keeps 0.5 s before the speech it finds, so a longer lead would be cut back by the server, at a place its detector chooses. A share above 0 makes the clip depend less on how much faint sound comes before the "re", but the measurement decides. The pick then has to keep `voice_forms_it`, `voice_samples_it` and the E2E scenarios green; a failure there drops it for the next one.

**Where.** A pure function in a new `command-spotting/src/engine/clip.rs`, next to `pause.rs` and using its loudness helpers. It takes the audio from the end of the previous utterance to the clip end, where the pre-roll starts, and the shape (onset share, lead, target level, cap); it gives the samples and the origin. `close_segment` calls it for short segments and hands the samples to `ask` in place of `RecentAudio::between`. The shape sits in `EngineCfg`, its default the chosen values and no command line flag, so the measurement runs each variant through the real engine. Each such clip logs its lead and gain at debug level next to its job line.

**Not changed:** the audio fed to the keyword spotter and the detector (the clip is a copy); the dictation's own audio (the app records it); the head and window checks and the last 2.5 s of a long utterance; the clip of a phrase no segment took (asked only for a phrase the keyword spotter found, and cut around it); the clip end; the request.

**Glossary:** `docs/ubiquitous-language.md` gains *Confirmation clip*: the audio a confirmation sends; for a short utterance, a fixed lead before its first sound, raised toward a normal level when quiet.

**Alternatives:**

- *Take "let's start" or "start" as "restart" during a dictation.* Rejected (proposal): both are ordinary words in a dictation, and a wrong restart throws the dictated text away, while a missed one is only said again.
- *A second hearing as the fix.* Rejected as the fix: one more request and its delay for every near miss, and a second chance for a quiet "let's start" to become a restart. Kept as the fallback (D3).
- *A lead of digital silence only*, dropping the audio before the first sound. Rejected: a faint first word below the onset level ("let's") would go, so "let's start" could come out as "start" or "restart".
- *A lead over 0.5 s.* Rejected: whisper-server's detector cuts it back to 0.5 s before the onset it finds.
- *Turning whisper-server's detector off or changing its padding.* Rejected: the flags are shared with every dictation request, and dictation must not change (proposal § Out of scope).
- *Raising every clip to one level.* Rejected: it changes clips that are heard right today; only quiet ones are raised.

### D3. Fallback: a second hearing of a near miss — only if no variant of D2 is right in every cell

- **Trigger.** The confirmation of a short utterance heard no command, and its transcript is one sentence of at most three words (by the matcher's words: "Let's start." is "let", "s", "start") that ends in a listed near miss of the one-word form of a command the state accepts. The list sits next to `ALIASES` in `matcher.rs` and starts with what the measurement shows for the user: "start" for "restart dictation". A list, not an edit distance, so "cancelled" never asks for a cancel.
- **Not the keyword spotter.** A trigger on the command the keyword spotter found would not work: the spotter has no one-word keywords (§ Context): it reports nothing for a lone "restart". D1 records the spotter's phrases to confirm this.
- **Second clip.** The same utterance cut with the second lead the measurement found right for the recording (D2's clip with that lead, or the pre-roll cut if D2 is not built), sent with the same request. If confirmations still retry at a higher temperature, this one asks for no retry, so chance never decides a restart.
- **Counts** only when the second transcript as a whole is the near-missed command (`phrase_in`); any other answer, or a second hearing given up, is no command. The segment's events stay held until the second answer (the outbox moves the hold to the second job).
- **Cost.** One more confirmation, only for a near miss: about 0.15 s on an idle service (p95 of the user's command clips without retry, `limit-confirmation-fallback`), well within the confirmation deadline.
- **Guard against "let's start" becoming a restart.** Only a short utterance ending in a listed word asks; "let's start" in or at the end of a longer sentence never does. The second lead must be one at which the measurement heard "start" and "let's start" alone, at both levels, as themselves 10 times out of 10, and the negative scenarios run with it.
- If D3 does not make the recording heard right in every cell either, stop and ask the user: nothing else is in scope.

### D4. The exclusion goes

`UNSTEADY_RECORDINGS`, the filter in `recorded_form` that uses it and their comments are removed. `recorded_form("restart")` then gives the restart at 27.23 s, the first "restart" of `session-2`, so the restart rows of `fix-short-pause-after-command`'s outline "The user's recorded command alone, a short pause, then speech" run on it. If D1 shows the failure, the removal goes into the reproduction commit, since it turns those rows red; otherwise it lands with the tests.

## Test levels

| Spec scenario (voice-control) | Test level |
|---|---|
| Every recorded command of the user is recognized | adapter integration, unchanged: `voice_samples_it`, `voice_forms_it` (every labelled form, the 27.23 s restart included), in `just e2e-smoke`; with D2 they hear every recorded form through the new clip |
| A short Russian phrase is no command | E2E `.feature`, the existing `@smoke` scenario |
| The user's quiet restart after any pause before it | E2E `.feature` (`@smoke @voice_samples`) Scenario Outline: a dictated sentence, 0.3, 0.4 … 0.8 s of silence, the recorded restart, while dictating: "restart dictation" once. With D2: engine integration in `engine_it.rs` (a `Hear` that keeps the clips, no service): the clip confirming it is the same, from its start through the command, after every pause, for the recording and for a quiet synthesized "restart"; unit tests of `engine/clip.rs` |
| The user's quiet restart, a short pause, then speech | E2E `.feature` (`@smoke @voice_samples`) Scenario Outline: the recorded restart, 0.3, 0.45 and 0.6 s of silence and a sentence, while dictating: "restart dictation" once; the restart rows of `fix-short-pause-after-command`'s outline run on it too (D4). With D2: the engine test above, with speech after |
| A command said in a quiet voice is recognized | E2E `.feature` (`@smoke`) Scenario Outline: the six command phrases and six short forms at the quiet level, each in the state that accepts it; the user's own quiet commands are in `voice_forms_it`; with D2's gain, its unit tests |
| Stop dictation without a dictation is ignored | component and E2E `.feature`, existing, unchanged |
| Clear during a dictation is ignored | component `.feature`, existing, unchanged |
| Start or let's start on its own during a dictation is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "start" and "let's start" after a sentence, at `say`'s level and at the quiet level, while dictating: no command. Unit: `matcher.rs` takes "Let's start." as no command while dictating |
| Start ending a dictated sentence is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "we are all here, let's start" and "the meeting is about to start", at both levels |
| Start inside a dictated sentence is ignored | E2E `.feature` (`@smoke`) Scenario Outline: "let's start with the budget" and "we start at nine tomorrow", at both levels |

- The new E2E scenarios sit under the rules of their requirements in `e2e-test/tests/features/voice_control.feature` and stream at once through the real command spotting service, as "A command word inside a sentence is still no command" does (`carried_out_by_app` with the dictating phrases): no command there means the dictation was never left, so it keeps running. The restart's steps pick the recording by its label (session and start time), not as the first match.
- **Quiet level:** `say` speech scaled so its loudest 10 ms window has an RMS of 0.005, the level of the user's quietest labelled commands (0.0045 at the lowest); a helper in `e2e-test/src/voice_commands.rs`.
- **Unit tests of `engine/clip.rs`** (D2): the first sound is found at the same place in the audio wherever the audio starts; the lead is cut to its length, and padded with silence when the previous utterance ended closer; nothing louder than the onset level is left out; the gain applies only below the target, never above the cap, never below 1; the origin counts the padding.
- **D3, if built:** unit tests of the near-miss list in `matcher.rs`; engine integration in `engine_it.rs` with a scripted `Hear`: "Let's start." then "Restart." gives one restart, "Let's start." twice gives none, a long utterance ending in "let's start" asks no second hearing.
- **Ports:** the E2E scenarios and the voice-sample tests reach the command spotting service on its port, so `just check` and `just full-check` run on ports of their own (`VOICEPUNCHER_STT_PORT=9481 VOICEPUNCHER_LLM_PORT=9480 VOICEPUNCHER_KWS_PORT=9482`): the recipes reuse a service already running on 8082, which may be an older build.
- **Tests only:** if D1 shows the recording already works, the new scenarios pass at once. Each is then seen failing once for the expected reason (AGENTS.md § TDD), with one of the three changes undone locally until the restart is heard as "Let's start." again — the prompt first: the confirmation request without it — and restored before the commit.

## Risks / Trade-offs

- [D2 changes the clip of every short utterance, not only quiet ones, so a command heard right today could flip] → the lead is picked with passing neighbours, the gain never touches a normal voice, and `voice_forms_it`, `voice_samples_it` and the E2E scenarios hear every recorded and synthesized command through it before the merge.
- [Raising a quiet clip raises its noise, so room noise could be heard as words] → the gain comes only if the measurement needs it, is capped (a room at 0.0001–0.0003 stays below 0.003), and only a segment the detector took for speech is confirmed; the no-command scenarios (corpus speech, short Russian phrases, a command word inside a sentence) and the quiet negatives run with it.
- [A steady clip can be steadily wrong] → D1's sweep finds that out for each variant; D3 is the fallback, and asking the user the end.
- [A faint first word more than the lead before the first sound is cut] → only audio below the onset level is ever cut, the lead is at least 0.2 s, and the quiet "let's start" negatives run with every variant.
- [The E2E suite talks to an older command spotting service on 8082] → ports of their own for `just check` and `just full-check`.
- [The three changes land differently: the prompt kept off for confirmations, other names] → the re-read list in § Context; D1 measures what landed.
- [D3 lets a quiet "let's start" become a restart when its second hearing says "Restart."] → D3's guard, and D3 is built only if D2 fails.

## Migration Plan

None: no setting, protocol or data change; the clip's shape is a default in the code. Rolling back is reverting the change's commits, the tests and the removed exclusion with them.

## Results

To be filled by the implementation: D1's tables before any change of the clip and for each D2 variant tried, the chosen variant and its values, and the exit taken.
