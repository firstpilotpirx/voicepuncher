# Design

## Context

`chunking::next_cut(samples, sample_rate, chunk_start, cfg)` (before this change: `last_cut`) is the one chunk rule in the core, used by `split_into_chunks` (recording files, 16 kHz) and by the live watcher in the cpal recorder (native rate, e.g. 48 kHz, on the growing buffer). Today it returns the start of the latest 20 ms frame below `cfg.vad_rms_threshold` within `vad_lookback_secs` before the hard boundary. See proposal.md § Why for the effect and the spike numbers.

`[transcription]` is `deny_unknown_fields`; the config loader (`app/src/config.rs`) already refuses the removed `[whisper]` section with a hint per key (`transcription::reject_removed_whisper_section`).

## Goals / Non-Goals

**Goals:**
- One pure function in the core decides the cut for both callers, rate-agnostic, with no new configuration.
- Rule constants live in `chunking.rs`, named, each with its reason.

**Non-Goals:**
- Voice activity detection by a model or a spectral feature; loudness per frame stays enough (spike: 98 % of seams in a pause).
- Changing the overlap, threshold value or lookback.

## Decisions

### D1. Longest quiet stretch, not the latest

`next_cut` measures 20 ms frames from the chunk start to the hard boundary, finds the runs of quiet frames inside the lookback, keeps runs of at least `MIN_PAUSE_MS = 160`, and returns the middle of the longest (ties: the latest). No such run → the hard boundary.

- *Alternative: latest stretch ≥ N ms.* Spike: 91–93 %; it prefers a short gap near the boundary over the sentence pause a second earlier, and misses under high noise (50–72 %).
- *Alternative: N = 250 ms (proposal draft).* Spike: 69–81 %; `say` leaves 150–250 ms between most phrases, so many lookbacks hold no 250 ms run and the cut falls back to the hard boundary.
- 160 ms stays above the 120 ms microphone dropouts of the seam matrix.
- Cutting in the middle leaves silence on both sides, so neither chunk ends or starts mid-word even with frame-level jitter.

### D2. Quiet level from the noise floor

`quiet level = max(MIN_QUIET_RMS = 0.002, min(NOISE_FLOOR_FACTOR = 2 × the NOISE_FLOOR_PERCENTILE = 10th percentile, SPEECH_LEVEL_SHARE = 0.5 × the median))`, both percentiles of the frame RMS from the chunk start to the hard boundary.

- The window is the chunk so far (≈ 30 s), not the 5 s lookback: a lookback without any pause makes its own 10th percentile a speech level, and loud frames would count as quiet. The chunk window holds the pauses of the last half minute.
- The floor `0.002` keeps a recording with digital silence (dropouts, lead-in; 10th percentile ≈ 0) from needing exact zeros, and stays below quiet speech (spike: median frame RMS 0.01–0.02).
- Spike, 10th percentile: noise low 0.005, noise high 0.024, quiet 0.0004, clean ≈ 0.002 → levels 0.010, 0.048, 0.002, 0.004.
- *Alternative: keep a configurable absolute level.* Rejected in the proposal: no fixed number fits noisy and quiet recordings at once.
- The cap at half the median keeps speech without any pause from counting as quiet: there the 10th percentile is itself a speech level, and twice it would cover every frame. Spike: the cap changes no cut of the matrix (98 %); at 0.25 or 0.35 × the median, high noise drops to 50–61 %.
- Live cost: one sort of ≤ 1 500 frame values (48 kHz: same frame count) once per cut — negligible.

### D3. `vad_rms_threshold` removed and reported

`ChunkingCfg::vad_rms_threshold` and `TranscriptionCfg::vad_rms_threshold` go. The loader refuses `[transcription].vad_rms_threshold` before deserializing, with a message that pauses now follow the recording's background noise level and the setting can be deleted (a new `reject_removed_transcription_keys` next to `reject_removed_whisper_section`). In the `[whisper]` migration hints, `vad_rms_threshold` says "delete it" instead of "move it to [transcription].vad_rms_threshold".

- *Alternative: accept and ignore.* A silently ignored knob misleads; the existing requirement already refuses outdated settings with a hint.

### D5. The chunk threshold bounds the whole chunk

The hard boundary is `chunk start + threshold`, where a chunk starts `overlap` before the previous cut (the first one at 0); `next_cut` takes the chunk start instead of the previous cut. A recording file and a live dictation start a new cut only while more than a threshold of audio follows the chunk start, so the last chunk is never longer than the threshold either.

- Found at the full-check: the service decodes 30 s windows, and the audio of a 32 s chunk (2 s overlap + 30 s up to the cut) beyond 30 s was lost — the end of a middle chunk vanished ("At the end of the day I updated the documentation and sent a", 13 % missing). The old rule cut near the threshold, so recordings of 35–60 s had two chunks of ≤ 30 s; cuts in earlier pauses give them a middle chunk. Dictations over 60 s lost words this way before as well.
- The live watcher waits for a threshold of audio after the chunk start instead of after the previous cut: a cut comes up to one overlap (2 s) sooner.
- After the stop, the live watcher still cuts while more than a threshold of audio follows the chunk start, so its final chunk fits the threshold as well.
- A pause is searched after the chunk's overlap only, so every cut moves past the previous one.
- *Alternative: lower the default threshold to 28 s.* Rejected: a configured threshold of 30 would still produce 32 s chunks.

### D4. Measuring pauses in the matrix test

`e2e-test::speech` gains `synthesize_with_clean(lang, condition, lead_in) -> (degraded, clean)`; `clean` is the same speech before `degrade`. A cut is "in a pause" when the clean frame at the cut is below 0.003 RMS inside a run of at least 100 ms. Cuts at the end of the recording are not seams and are skipped. The phone-line condition is left out: its filter shifts the signal against the clean copy.

### Test level per spec scenario

| Scenario | Level |
|---|---|
| Dictation longer than the chunk threshold; Chunks transcribe the overlap differently; Chunks share no words | component `.feature` (unchanged) |
| Recording file longer than the chunk threshold; Long speech under difficult recording conditions | E2E `.feature` (unchanged; `@smoke`, `@seams`) |
| Chunk seams fall on different words | adapter integration `chunked_transcription_it.rs` (unchanged) |
| Chunk ends in a pause rather than in a quiet moment of a word | unit, `chunking.rs` |
| Chunk ends in the longest pause | unit, `chunking.rs`; the minimum length (a 170 ms pause is found) and the tie-break (the latest of two equal pauses) each have a unit test of their own |
| Pause is found under background noise | unit, `chunking.rs` |
| Quiet speech is not taken for a pause | unit, `chunking.rs` |
| Short audio dropout is not a pause | unit, `chunking.rs` |
| Speech without a pause ends at the threshold | unit, `chunking.rs` (`should_cut_at_the_threshold_when_the_lookback_holds_no_pause`, renamed from `should_cut_at_hard_boundary_when_no_silence_found`) |
| No chunk is longer than the chunk threshold | unit, `chunking.rs` (`split_into_chunks` on a recording more than twice the threshold) |
| Most seams of synthesized speech fall in pauses | integration `app/tests/chunk_cuts_it.rs` (macOS `say`, no service) |
| Config still points at a model file | E2E `.feature` (unchanged) |
| Config still sets a fixed silence level | unit, `config/transcription.rs` + E2E `@smoke` `.feature` |

The cut scenarios are unit tests on synthetic samples: the cut position is not observable through the app's output, only through the rule.

## Risks / Trade-offs

- [Real microphones: room tone that varies within a chunk] → the 10th percentile over 30 s tracks the quieter part; a louder stretch hides its pauses, and the cut falls back to the hard boundary, as today. The seam merge (0.3.1) still repairs it.
- [Breathing or clicks inside a pause split it into shorter runs] → 160 ms is short enough that a sentence pause still yields a run; the longest one wins.
- [A user config with `vad_rms_threshold` stops starting] → the message says to delete the line; the user's own config does not set it.
- [The cut moves up to 5 s earlier than today] → the next chunk is longer by the same amount; chunk length stays ≤ threshold + overlap, unchanged bound.
