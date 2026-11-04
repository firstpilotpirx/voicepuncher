## Context

Two losses inside the command spotting service, reproduced from the command history (proposal § What Changes): `decide/` settles a one-word command on decoded time alone; `session/utterance.rs` finishes the recognizers' streams at the close with no audio after the gate's last speech. The live entries become session 6.

## Goals / Non-Goals

- Goals: no "stop" from a word the gate still hears speech after; a quiet "старт" the gate cut short decoded at the close; both guarded on the user's own voice; the latency bounds kept.
- Non-goals: the gate's thresholds, the bench's `--fast` mode, the English recognizer's lag.

## Decisions

**D1. A lone one-word command settles at the gate's pause.** In `settled`, `Moment::Decoding { pause: None, .. }` never settles a one-word command; with `pause: Some(p)` it settles when `p − end ≤ SPEECH_AFTER_SECS` and `now − end ≥ SETTLE_SECS`. `ended` and `Closing` stay. A sync the app asks for stands for the pause of a command said alone only: `Utterance::sync` sets `synced` on the moment while it is answered, `settled` takes it like a pause begun right after the word (decoded `SETTLE_SECS` past it, as before this change), and `ended` ignores it, so a command word ending a partial sentence ("we should stop", "I will paste") is not reported at the syncs the app asks for inside a sentence (dictation chunk seams); the existing requirement that a command said right before a sync is reported before the sync is answered holds. Kept out: a longer `SETTLE_SECS`, which delays every lone command and still fires inside a slow sentence. The pause begins about 0.1 s after the word, within the 0.2 s already waited for, so the 0.3 s bound holds. A doubled form inside running speech ("stop stop" in a sentence) no longer fires on its first word: its second word is reported once the recognizer emits it, which takes the speech past it (0.56 s measured in `e2e-cues`), so the bound of a doubled form said inside a sentence moves from 0.5–0.6 s to 1 s (the user's decision of 2026-11-04); a doubled form followed by silence keeps its 0.5 s. A doubled one-word form reports the end of its first word (`finding`): before this change the first word fired alone and the dictation began at its end; with both words needed, the second word's end stamp of the English recognizer, which emits late, lay inside the sentence after it and the user's recorded "restart restart" followed at once by a sentence lost the sentence's first word; the app already removes a doubled command word from the head of the text.

**D2. A tail of silence before the final decode.** `Utterance::close` feeds `CLOSE_SILENCE_SECS` of silence through `silence()` (left out of the word times) before `Decoder::finish`: 1.0 s: 0.5 s loses one of the two quiet starts of session 6, 0.8 s decodes both, and 1.0 s keeps a margin. Cost: the close decodes the tail too (Russian about 20 ms, English up to 0.6 s); only a command settled at the close is delayed; `e2e-cues` and the bound scenarios measure it. Refuted or kept out: a gate reset at the close (tried: a fresh detector hears the same windows); a longer `min_silence` (changes "an utterance ends after 0.3 s of silence"); a second flush (live the gate did not hear the word's burst).

**D3. The reproduction of D2 replays the gate's hearing, not the gate.** The live gate stopped hearing at 0.97 s where the offline gate hears the clip to its end (window alignment and detector state differ), so a replay through the real gate does not reproduce the loss. `StreamingEngine::session_with_gate(conn, gate)` builds a session on the engine's models with a scripted gate that hears speech until the entry's `speech_end`. The test (`command-spotting/tests/it/streaming/`, `needs_streaming_models::needs_voice_samples`) feeds the session-6 stretch in 10 ms frames as the app does.

**D2a. The close waits for no decode.** The second `kit verify` failed `e2e-cues`: "press enter" after a sentence came 1.44 s after the words with every core busy, over its 1.2 s bound, as D2's tail was decoded inside the blocking `Utterance::close` before any report. Now `Session::close` only feeds the tail, ends every stream (`Utterance::begin_close`) and reports the speech end at once; the utterance moves to `Session::closing`, every later `poll`, `sync` and frame collects what its recognizers decoded of the tail (a command emitted before the final decode is reported from that update, at the pause), and once every recognizer finished (`Decoder::finished`; the decoding thread wakes the server after its final update too) the utterance is logged and kept in the history (`Keeper`). Kept from the blocking close: a finished update whose last word may be the end of a longer command word ("start" of a quiet "restart") is held back (`Utterance::held`) until the other recognizer finished, so the sorted reports of both still put the whole word first; a sync releases it. A command may now be reported after the speech end; the app carries out nothing at the bounds of speech, and the tests of the session sync before they read the close's reports.

**D4. Session 6 from the command history.** `~/.voicepuncher/voice-samples/session-6.wav` and `session-6/forms.jsonl` in the form of sessions 2–5, assembled by a scratchpad script: 1.5 s of silence, each entry's `audio.wav`, 1.5 s of silence, in this order:

| entry | said | state | commands |
|---|---|---|---|
| `2026-11-04T13-58-55.846Z-86346-30006-684` | чтобы зайти в гетлаб | dictating | none |
| `2026-11-04T13-59-03.762Z-86346-30006-687` | чтобы зайти в гетлаб | dictating | none |
| `2026-11-03T11-12-03.340Z-865-4-685` | старт | standby | start dictation |
| `2026-11-02T11-07-47.633Z-45180-7594-910` | старт | standby | start dictation |

The gate's hearing of 685 and 910 (speech end 0.97 s and 1.05 s, close 1.37 s and 1.46 s into the entry) goes into the D3 test as constants. Entry `…-37736-7` (RMS 0.001, inaudible) is left out. The audio of entry 684 ends in the first 0.25 s of the user's real "стоп", cut by the close: that stretch (3.8–4.05 s of the session) is a row marked `unclear`, so no clip holds it and the clip of the sentence ends before it (through the real gate and the tail of silence, the fragment alone decodes as "stop").

## Test levels

| Spec scenario | Test |
|---|---|
| A Russian sentence heard as opening with stop is no command | `decide` unit test from entry 684 (red first); E2E `@smoke @voice_samples` scenario, the session-6 stretch in real time through `run_clip` |
| The user's quiet start cut short by the gate is recognized | D3 integration test (red first) |
| Every recorded command of the user is recognized | `voice_forms` on session 6 (existing test, new rows) |
| A short Russian phrase is no command; The user's quiet restart after any pause before it; The user's quiet restart, a short pause, then speech; A command said in a quiet voice is recognized; A quiet clear all text clears the text | existing E2E scenarios, unchanged, rerun in `just check` |
| Start said alone is reported within its bound; Stop said alone is reported at the pause after it; Stop opening a sentence is still no command; Press enter after a sentence is reported soon after the pause | existing E2E scenarios, unchanged, rerun in `just check` (the bounds measure D1 and D2) |
| The user's recorded commands are reported within their bound | existing `@wip` scenario, by hand with `just streaming-bench --recorded` (task 4.2) |

## Risks

- D2's tail may not decode 910 at any length: then its row gets a note, the D3 test covers 685 alone, and the report says so.
- `recognize-glued-enter-mishearings` released 0.46.1 while this change ran: the target moved to 0.46.2 (`kit version-check`).
