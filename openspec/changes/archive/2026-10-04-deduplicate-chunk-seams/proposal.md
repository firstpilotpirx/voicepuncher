# Proposal

## Why

Long dictations sometimes paste the same phrase twice, and the two copies often differ slightly ("миллион граней. миллион граней, и мы…", "своего рода чека, Но своего рода чекап…"). The bug is intermittent: it appears or not depending on which word the chunk seam falls on.

Root cause: a dictation longer than the chunk threshold (30 s) is split into chunks that overlap by 2 s, each chunk is transcribed on its own, and the texts are joined by dropping the longest run of words that is **identical** at the end of one chunk and the start of the next. The speech recognition service rarely transcribes the same 2 s of audio identically in two chunks. Punctuation, case, spelling ("раунд-робинам" / "раундробином"), a word cut at the chunk edge ("чека" / "чекап"), or a different reading of the words ("30-30 дней" / "30 километров в день") all defeat the exact match, so nothing is dropped and the overlap is pasted twice.

Measured against the running whisper-server, cutting audio exactly as the live recorder does:

- 10 corpus recordings longer than 30 s: duplicates in 8.
- Speech synthesized with macOS `say` in Russian and English, 13 conditions each (speech rate 140/190/260 wpm, tempo ×0.8/×1.25, pauses between sentences, pauses between words, low and high background noise, phone-band audio, quiet, clipped, audio dropouts): duplicates in 18 of 26.
- Transcribing the same recordings in one pass: no duplicates.

No test catches this. The unit tests of the merge use overlaps that are identical word for word. The E2E accuracy run transcribes recording files, and file mode sends the whole recording in one request, so chunking never runs in any E2E test. The spec scenario "Dictation longer than the chunk threshold" has no `.feature` scenario.

## What Changes

- **Seam merge tolerates differing transcripts of the overlap.** Instead of an exact word match, the merge aligns the end of the text so far with the start of the next chunk's text. Words are compared after case, punctuation, hyphens and `ё`/`е` are normalized, and near-identical words count as a match: small spelling differences, or a word that one chunk cut short. The overlap is kept once. When the two texts share no confident overlap, nothing is dropped.
- **Recording files are chunked like dictations.** A recording file longer than the chunk threshold is split, transcribed chunk by chunk and merged with the same rules as a live dictation. The live recorder and file mode use one chunk-boundary rule (silence search before the hard boundary, then the overlap) that moves from the microphone adapter into the application core. This makes the seam behavior testable end to end with recording files. Side effect: long recordings no longer lose words that whisper-server drops in its own single-pass windowing (measured: 18 of 165 English words lost in one pass, 3 when chunked).
- **Tests on every level, in Russian and English:**
  - unit: dozens of real seam pairs captured from whisper-server output (both languages, all conditions above), plus rules for punctuation, case, hyphens, cut-off words, disjoint chunks and legitimately repeated words;
  - component `.feature`: the pipeline merges chunk transcripts that disagree on the overlap, in both languages;
  - adapter integration: the real speech recognition service transcribes chunked synthesized speech at several seam positions;
  - E2E `.feature`: the app transcribes long Russian and English recording files under every condition above without repeating a phrase or losing words.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: "Long recordings are transcribed in chunks" now covers recording files as well as dictations, and states that differing transcripts of the overlap are merged once and that no word is dropped where chunks share none.

## Impact

- **Stage:** current PoC → target PoC (bug fix).
- **Version:** PATCH: `0.3.0` → `0.3.1`.
- **Scope:** the seam merge, the chunk-boundary rule (moved into the core), chunked transcription of recording files, tests at unit, component, adapter integration and E2E level, the `justfile` recipe that runs the seam E2E matrix, docs.
- **Acceptance bar:**
  - Every seam-condition E2E scenario (Russian and English × 13 conditions) passes: no word pair occurs more often in the pasted text than in the spoken text, and at most 10 % of spoken words are missing.
  - The adapter integration seam sweep (Russian and English, several seam positions) shows no duplicated phrase.
  - `just full-check` passes; the corpus mean WER does not get worse than on `main`.
- **Out of scope:** changing the chunk threshold, the overlap length or the silence detector (the 20 ms quiet frame it looks for can fall inside a word; the tolerant merge makes that harmless, tuning it is a separate change); asking the speech recognition service for word timestamps (whisper-server runs with timestamps off); streaming partial transcripts; duplicates that the speech recognition service produces inside a single chunk.
- **Code:** `app/src/core/application/chunking.rs`, `app/src/core/application/pipeline.rs`, `app/src/adapters/secondary/recording/cpal/mod.rs`, `app/tests/`, `e2e-test/`, `justfile`, `docs/ubiquitous-language.md`, `AGENTS.md`/`README.md` where they describe checks.
- **Dependencies:** none new.
