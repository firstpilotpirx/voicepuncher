# Proposal

## Why

A recording longer than the chunk threshold is cut where the chunk rule (`chunking::next_cut`) finds the latest 20 ms frame quieter than an RMS of 0.01 within the 5 s before the hard boundary. A single quiet 20 ms frame is not a pause: it occurs inside words (stop consonants, between syllables). In the `deduplicate-chunk-seams` investigation almost every cut of synthesized speech fell 0.02–0.2 s before the hard boundary, in the middle of speech, not in one of the 1.5 s pauses a few seconds earlier. Recorded effects at the seams:

- the last word of a chunk is cut short: "оплат" / "оплаты", "чека" / "чекап", "out of da" / "out of date";
- whisper makes up words where a chunk's audio stops mid-word: "Вечером мы обсуждали, что мы не можем пересматривать список." in place of "…как разбить большой модуль";
- with background noise above the fixed level no frame counts as quiet, and the cut falls at the hard boundary, mid-word; with quiet speech (median frame RMS 0.01–0.02) frames inside words count as quiet.

A spike on the synthesized seam matrix (Russian and English, 10 of the 11 conditions, 9 lead-ins, 30 s threshold, 5 s lookback) measured where each rule cuts, against the pauses of the clean speech:

| Rule | Cuts in a pause |
|---|---|
| today: latest frame below 0.01 RMS | 28 % (normal pace: 5 %) |
| latest quiet stretch ≥ 200 ms, level from the noise floor | 91 % |
| **longest quiet stretch ≥ 160 ms, level from the noise floor** | **98 %** (every miss is a cut at the end of the recording, not a seam) |

Since the `deduplicate-chunk-seams` fix the seam merge tolerates edge errors, so they no longer duplicate text. But every cut through a word still costs recognition quality at the seam and leaves the merge to repair it.

## What Changes

- The chunk rule cuts in a **pause**: the longest quiet stretch of at least 160 ms within the lookback (the latest of equally long ones), cut in its middle. Without one, the hard boundary stays the fallback. 160 ms keeps the 120 ms microphone dropouts of the seam matrix from counting as pauses; `say` leaves only 150–250 ms between most phrases, so a longer minimum misses them.
- The quiet level follows the recording's own **noise floor**: twice the 10th percentile of the 20 ms frame loudness of the chunk so far, but at most half its median (so speech without pauses never counts as quiet) and at least an RMS of 0.002. Pauses are found under the matrix's high background noise and in quiet speech alike.
- `[transcription].vad_rms_threshold` is removed: the level is no longer configured. A configuration that still sets it is refused at startup with a message saying pauses now follow the recording's noise floor and the setting can be deleted.
- No chunk, its overlap included, is longer than the chunk threshold: the hard boundary counts from the chunk's start, not from the previous cut. A 32 s chunk (2 s overlap + 30 s) loses its audio beyond the service's 30 s window; cuts in earlier pauses made such chunks common (found at the full-check, scope extended by the user).
- Live dictations and recording files keep sharing the one rule in the core.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`:
  - "Long recordings are transcribed in chunks" gains the rule that a chunk ends in a pause of the speech when the lookback holds one, also under background noise and in quiet speech.
  - "Outdated speech recognition settings are reported" also refuses the removed `vad_rms_threshold` setting.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH, target `0.5.2` (from `0.5.1`; planned from `0.4.1`, `main` moved on during the change).
- **Scope:** `next_cut` and `ChunkingCfg`, the config key `vad_rms_threshold` and its migration message, `app/config.example.toml`, the glossary, unit tests for the cut, the seam tests that already exist.
- **Acceptance bar:**
  - Unit: a 160 ms pause inside the lookback is chosen over isolated quiet frames and over a shorter quiet stretch; a 120 ms dropout is not a pause; noise at the level of the `noise_high` condition does not hide a pause; quiet speech does not count as a pause; no pause → hard boundary.
  - Integration (no service needed, speech from macOS `say`): on the synthesized seam matrix (Russian and English, every condition but the phone line, several lead-ins), at least 80 % of cuts fall in a pause of the clean speech.
  - Unit: no chunk of a recording more than twice the threshold long is longer than the threshold.
  - `just e2e-seams` stays 22/22; the integration seam sweep shows no repeated phrase and no more lost words than on 0.4.1; corpus mean WER is not worse than on 0.4.1.
- **Out of scope:** the overlap length, chunk threshold value and lookback; dropping the overlap when the cut is in a pause; speech recognition settings and word losses inside a chunk (see `keep-words-after-pauses`).
- **Code:** `app/src/core/application/chunking.rs`, `app/src/adapters/secondary/recording/cpal/mod.rs` (live watcher; needs a manual run), `justfile` (E2E recipes skip `@wip`), `app/src/config/transcription.rs`, `app/src/main.rs`, `app/config.example.toml`, `docs/ubiquitous-language.md`, tests.
- **Dependencies:** none.
