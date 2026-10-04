# Proposal

## Why

A recording longer than the chunk threshold is cut where the chunk rule (`chunking::next_cut`) finds the latest 20 ms frame quieter than an RMS of 0.01 within the 5 s before the hard boundary. A single quiet 20 ms frame is not a pause: it occurs inside words (stop consonants, between syllables). In the `deduplicate-chunk-seams` investigation almost every cut of synthesized speech fell 0.02–0.2 s before the hard boundary, in the middle of speech, not in one of the 1.5 s pauses a few seconds earlier. Recorded effects at the seams:

- the last word of a chunk is cut short: "оплат" / "оплаты", "чека" / "чекап", "out of da" / "out of date";
- whisper makes up words where a chunk's audio stops mid-word: "Вечером мы обсуждали, что мы не можем пересматривать список." in place of "…как разбить большой модуль";
- with background noise above the absolute threshold no frame counts as quiet, and the cut falls at the hard boundary, mid-word.

Since 0.3.1 the seam merge tolerates these edge errors, so they no longer duplicate text. But every cut through a word still costs recognition quality at the seam and leaves the merge to repair it.

## What Changes

- The chunk rule cuts in a **pause**: the latest stretch of at least ~250 ms below the quiet level within the lookback, cut in its middle. Where there is none, the hard boundary stays the fallback.
- The quiet level follows the recording's own **noise floor** (for example, a margin above a low percentile of frame loudness) instead of a fixed 0.01 RMS, so pauses are found in noisy and in quiet recordings alike.
- Live dictations and recording files keep sharing the one rule in the core.
- Exact minimum pause length and noise-floor rule are settled in design.md, measured on the seam matrix.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: "Long recordings are transcribed in chunks" gains the rule that a chunk ends in a pause of the speech when the lookback holds one, also under background noise.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH: `0.3.1` → `0.3.2`.
- **Scope:** `next_cut` and its configuration (`vad_rms_threshold` may become a noise-floor margin), unit tests for the cut, the seam tests that already exist.
- **Acceptance bar:**
  - Unit: a 250 ms pause inside the lookback is chosen over isolated quiet frames; noise at the level of the `noise_high` condition does not hide a pause; no pause → hard boundary.
  - On the synthesized seam matrix (Russian and English, 11 conditions), at least 80 % of cuts fall in a pause of the speech, measured against the `say` sentence and word pauses.
  - `just e2e-seams` stays 22/22; the integration seam sweep shows no repeated phrase and no more lost words than on 0.3.1; corpus mean WER is not worse than on 0.3.1 (0.065).
- **Out of scope:** the overlap length and chunk threshold; dropping the overlap when the cut is in a pause; speech recognition settings and word losses inside a chunk (see `keep-words-after-pauses`).
- **Code:** `app/src/core/application/chunking.rs`, `app/src/config/transcription.rs`, `app/config.example.toml`, tests.
- **Dependencies:** none.
