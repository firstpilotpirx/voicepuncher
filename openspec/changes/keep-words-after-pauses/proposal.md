# Proposal

## Why

The speech recognition service drops whole phrases of long speech, mostly after pauses, and sometimes makes up others. The merge is not the cause: in the `deduplicate-chunk-seams` seam sweep it lost at most 0.9 % of the words the chunks recognized, while the chunk transcripts themselves missed up to 15 % of the spoken words. Recorded cases (whisper-server, large-v3-turbo, greedy decoding, `-nf -sns -mc 0 -nt`):

| Speech | Missing | What the service did |
|---|---|---|
| English, 350 ms between words | 15 % | a chunk starts at "and saw…", dropping "Then I opened the server logs"; another ends "several small parts. interface", dropping "Each part should be responsible for one thing only and have a clear" |
| English, 1.5 s between sentences | 13 % | "At the end of the day I updated the documentation and sent a short report" became "Jordan reports as a project lead." |
| Russian, 350 ms between words | 10 % | "Ещё нужно написать тесты на возврат денег, потому что там" is missing |
| English, one 47 s file in one request (before 0.3.1) | 18 of ~165 words | the sentence "In the evening we discussed how to split the large payment module into" is missing |

People pause while they dictate, so these losses reach the pasted text. The cause is not known yet. Candidates:

- long silence inside a chunk derails whisper's decoding (it skips to later speech or hallucinates);
- the service's decoding settings: no temperature fallback (`-nf`), no-speech threshold, suppressed non-speech tokens;
- whisper-server's own voice activity detection (Silero, `--vad`), which is off and would drop silence before decoding;
- trimming or shortening long silences in the app before a chunk is sent.

## What Changes

- **First an exploration** (`/opsx:explore`): measure each candidate above on the existing seam matrix (Russian and English, 11 conditions, several seam positions) and on the corpus WER, with the chunk-level missing-word share as the main number.
- **Then** this proposal is completed with the chosen change: service launch flags in `just start-model-servers` and `bench.py`, or audio preprocessing in the app, or both. Specs, design and tasks follow the measurement.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: a new requirement that speech after a pause is transcribed — the raw transcript of long speech with pauses misses at most a set share of spoken words. The share is fixed after the exploration.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH (`0.3.x`) if only service flags or preprocessing change; decided with the design.
- **Scope:** to be fixed after the exploration; likely `justfile` (`start-model-servers` flags), `e2e-test/scripts/bench.py`, possibly a silence step in the core before transcription, and a missing-word bar in the seam tests.
- **Acceptance bar (draft):**
  - in every case of the integration seam sweep and the E2E seam matrix, the chunk transcripts miss at most 5 % of the spoken words (today up to 15 %);
  - no made-up phrase in the matrix (today "Jordan reports as a project lead.");
  - no repeated phrase at a seam (0.3.1 bar holds); corpus mean WER not worse than 0.065.
- **Out of scope:** another speech recognition model or engine; cloud services; the chunk cut point (see `cut-chunks-at-pauses`, which should land first because it changes where silences fall inside chunks).
- **Dependencies:** possibly the Silero VAD model file for whisper-server, if that candidate wins.
