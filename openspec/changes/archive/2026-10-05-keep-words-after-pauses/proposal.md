# Proposal

## Why

The speech recognition service drops whole phrases of long speech, mostly after pauses, and sometimes makes up others. The merge is not the cause: in the `deduplicate-chunk-seams` seam sweep it lost at most 0.9 % of the words the chunks recognized, while the chunk transcripts themselves missed up to 15 % of the spoken words. Recorded cases (whisper-server, large-v3-turbo, greedy decoding, `-nf -sns -mc 0 -nt`):

| Speech | Missing | What the service did |
|---|---|---|
| English, 350 ms between words | 15 % | a chunk starts at "and saw…", dropping "Then I opened the server logs"; another ends "several small parts. interface", dropping "Each part should be responsible for one thing only and have a clear" |
| English, 1.5 s between sentences | 13 % | "At the end of the day I updated the documentation and sent a short report" became "Jordan reports as a project lead." |
| Russian, 350 ms between words | 10 % | "Ещё нужно написать тесты на возврат денег, потому что там" is missing |
| English, one 47 s file in one request (before chunked file transcription) | 18 of ~165 words | the sentence "In the evening we discussed how to split the large payment module into" is missing |

People pause while they dictate, so these losses reach the pasted text.

**Cause, measured.** The service is started with `-nt` (no timestamps). Without timestamp tokens whisper.cpp decodes each 30 s window in one pass: when the decoder ends the text early, typically at a pause, whisper.cpp moves on by the whole window and the rest of the window's speech is never decoded. With timestamps it resumes after the last decoded segment. Every candidate was measured on the seam matrix (Russian and English, 11 recording conditions, 3 lead-ins, 66 chunked transcriptions) and on the corpus (100 recordings):

| Service setting | Worst chunk missing-word share | Longest run of dropped spoken words | Corpus mean WER |
|---|---|---|---|
| today (`-nf -sns -mc 0 -nt`) | 15.4 % | 13 | 0.063 |
| **without `-nt`** | **8.6 %** (audio dropouts) | **5** (one misheard phrase) | 0.067 |
| without `-nf` (temperature fallback) | 15.4 % | 13 | — |
| without `-sns` | 15.4 % | 13 | — |
| whisper-server VAD (Silero), without `-nt` | 36.2 % | 33 | — |
| quiet stretches > 600 ms cut to 300 ms in the app | 29.3 % | 32 | — |
| one 47 s request, today / without `-nt` | 16.4 % / 8.6 % | 18 / 5 | — |
| **without `-nt`, VAD only for silences > 2 s (`--vad -vsd 2000 -vp 500`)** | **8.6 %** | **3** (dropped-phrase metric) | 0.065 |

Without `-nt` the pause conditions drop from 12.6–15.4 % to 2.1–2.8 % missing words. What remains are misheard words and numbers written as digits, not dropped phrases. The corpus WER rises by 0.004 because of one recording of 93: a 33.8 s file cut mid-speech at 30 s, where the service now makes up a clause at the cut. Where the chunk rule cuts is the subject of `cut-chunks-at-pauses`.

Without `-nt` whisper also turns pure silence (1, 10 and 60 s) into "Thank you." instead of "you". The app's hallucination filter deliberately keeps "Thank you", which can be dictated, so silence would be pasted as "Thank you.". No decoding flag (`-nth`, `-nf`, `-sns`) changes that. whisper-server's own voice activity detection, set to remove only silences longer than 2 s with 0.5 s of padding, returns an empty transcript for silence. On the seam matrix it loses no more words than timestamps alone. The default VAD settings (100 ms minimum silence) are what lost up to 36 % of the words with pauses.

## What Changes

- The speech recognition service is started without `-nt` and with its voice activity detection limited to silences longer than 2 s (`--vad -vm <Silero model> -vsd 2000 -vp 500`), in `just start-model-servers`, in `e2e-test/scripts/bench.py` and in the README's manual start command. The other decoding flags stay: `-nf` and `-sns` made no difference in any case.
- `just models` also downloads the Silero VAD model (`ggml-silero-v5.1.2.bin`, ~0.9 MB) into `~/.voicepuncher/models/`; `start-model-servers` finds it there (override with `VOICEPUNCHER_VAD_MODEL`) and refuses to start the service without it. The model belongs to the service; the app never loads it.
- The integration seam sweep fails when its chunk transcripts drop a phrase of more than 5 spoken words, and new E2E scenarios with pauses between words or sentences fail when the printed text does. A dropped phrase is measured as the spoken words in a gap of the word alignment beyond the transcript words in the same gap, so misheard words do not count.
- The app joins the service's segment lines already. Its one change: an empty transcript from the service (VAD's answer to silence) means no speech, as a discarded hallucination does, instead of a service failure.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: a new requirement that speech after a pause is transcribed — the raw transcript of long speech with pauses drops no phrase of more than 5 consecutive spoken words; and a new requirement that an empty transcript from the service yields no text, not a failure.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH, target `0.5.1`.
- **Scope:** the empty-answer case in `app/src/adapters/secondary/transcription/openai/mod.rs`, `justfile` (`start-model-servers` flags and VAD model, `models`), `e2e-test/scripts/bench.py`, `README.md`, the seam metric in `e2e-test/src/speech.rs`, the seam sweep `app/tests/chunked_transcription_it.rs`, the E2E pause scenarios and their steps.
- **Acceptance bar:**
  - in every case of the integration seam sweep (chunk transcripts) and of the E2E pause scenarios (printed text), no dropped phrase of more than 5 spoken words (today up to 12 by this measure; 13 as a plain run of missing words in the table above);
  - the existing bars hold: no phrase repeated at a seam, at most 1 in 10 spoken words missing from the printed text;
  - corpus mean WER at most 0.070 (0.067 measured without VAD; 0.063 today);
  - silence of 1, 10 and 60 s still produces no text (the lone-"you" smoke scenarios).
- **Running services:** a speech recognition service that is already running keeps its old flags; the user restarts it (`just stop-model-servers whisper-server && just start-model-servers`) after the merge.
- **Out of scope:** another speech recognition model or engine; cloud services; VAD with its default settings and silence trimming in the app (both measured worse); where the chunk rule cuts (`cut-chunks-at-pauses`); numbers written as digits by the service.
- **Dependencies:** the Silero VAD model file for whisper-server (`ggml-silero-v5.1.2.bin` from `huggingface.co/ggml-org/whisper-vad`), downloaded by `just models`.
