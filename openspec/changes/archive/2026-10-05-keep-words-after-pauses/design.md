# Design

## Context

`just start-model-servers` starts whisper-server with `-bo 1 -nf -sns -mc 0 -nt`; `e2e-test/scripts/bench.py` and the README's manual start command use the same flags. With `-nt` whisper.cpp decodes without timestamp tokens, and after a decode pass it advances by the whole 30 s window, so speech after an early end of text is never decoded (proposal § Cause, measured). With timestamps, whisper-server still answers `response_format=json` with only the `text` field: the segments are joined with `\n`, and each starts with a space at a word boundary. `normalize_transcript` already removes those newlines, so the app's output keeps its form.

The seam tests measure the share of spoken words missing (`missing_word_share`, longest common subsequence of normalized words). That share mixes dropped phrases with misheard words and numbers written as digits ("2" for "two"), which put Russian at about 5 % even when nothing is dropped. A share bar cannot tell a dropped phrase from a misheard one.

## Goals / Non-Goals

**Goals:** speech after a pause reaches the raw transcript; the seam tests catch a dropped phrase.

**Non-Goals:** misheard words, numbers as digits; the chunk cut point (`cut-chunks-at-pauses`); silence detection in the app.

## Decisions

1. **Drop `-nt`, keep the rest.** Measured on the seam matrix and the corpus (proposal table). `-nf` and `-sns` changed no transcript, so removing them would only add risk. VAD with default settings and app-side silence trimming lost up to 36 % and 29 % of the words with pauses between words: both cut the short silences whisper uses to separate words, and both are rejected.
1a. **Service VAD only for long silence.** Without `-nt` silence comes back as "Thank you.", which the hallucination filter must keep (it can be dictated). whisper-server's VAD with `-vsd 2000 -vp 500` removes only silences longer than 2 s, keeps 0.5 s around speech, and returns an empty text for silence. It matched timestamps alone on the seam matrix (at most 3 dropped words). The model file is the service's: `just models` downloads it next to the other models, `start-model-servers` finds the newest `ggml-silero-*.bin` in `~/.voicepuncher/` or `~/.voicepuncher/models/` (`VOICEPUNCHER_VAD_MODEL` overrides) and exits with a message naming `just models` when it is missing. `bench.py` uses the same flags. Alternative — adding "Thank you" to the lone-hallucination list in the app; rejected, it would drop a dictated "Thank you".
1b. **An empty answer is no speech.** With VAD the service answers silence with an empty text. `OpenAiTranscriber::transcribe` treated that as an unusable answer (`EmptyTranscript`), so file mode exited with code 1. It now returns `Ok(None)`, as for a discarded hallucination; the pipeline already handles `None` as "no text".
2. **Measure dropped phrases by alignment gaps.** New `dropped_phrase_len(spoken, transcript)` in `e2e-test/src/speech.rs`: align the normalized words by longest common subsequence; for every gap between two aligned words, the spoken words in the gap minus the transcript words in the same gap are dropped words; return the largest. A misheard phrase ("по полю даты создания" → "попа людат и создание") fills its gap and counts 0. A dropped phrase, or one replaced by a shorter made-up one ("…sent a short report to the project lead" → "Jordan reports as a project lead"), counts its missing words. Measured: up to 12 with `-nt` (7 of 66 cases above 5), at most 3 without it. The bar is "more than 5 fails". Alternative: the longest run of missing spoken words; rejected, because a misheard 4–5-word phrase already reaches 5 and leaves no margin.
3. **Integration sweep checks the chunk transcripts.** `chunked_transcription_it.rs` asserts `dropped_phrase_len(spoken, chunk texts joined) <= 5` in the same loop as its seam checks. Joining chunk transcripts keeps the overlap twice. That can only fill a gap at a seam, and seams are not where the phrases were lost.
4. **E2E checks the printed text.** A new `Rule: Speech after a pause is transcribed` in `speech_recognition.feature`, tagged `@seams`, covers Russian and English with pauses between words and between sentences. Its Then step asserts the bar on the printed text, so the merge is covered too.
5. **Tests against the new flags without touching the user's service.** The user's whisper-server on port 8081 keeps the old flags until they restart it. The E2E recipes reuse a running service, so the Green runs set `VOICEPUNCHER_STT_PORT=8092` (8091 is the benchmark's). `start-model-servers` then starts a service with the new flags there and stops it afterwards. The Red runs use the service with today's flags.

## Test levels

| Spec scenario | Level |
|---|---|
| Speech with pauses between words or sentences | E2E `.feature` (`speech_recognition.feature`, `@seams`), plus unit tests of `dropped_phrase_len` |
| Speech after pauses wherever the seams fall | adapter integration (`chunked_transcription_it.rs`, seam sweep) |
| Service hears no speech | adapter integration (`transcription_openai_it.rs`, stub answers `{"text": ""}`), plus the existing `@smoke` scenarios that run silence through the real service |
| (existing) Silence produces a lone "you" | E2E `.feature` (`@smoke`), unchanged; it fails with timestamps on and no VAD, and passes with VAD |

## Risks / Trade-offs

- [A clause made up at a chunk cut mid-speech: 1 of 93 corpus recordings, WER 0.063 → 0.067] → the WER bar is 0.070; `cut-chunks-at-pauses` moves cuts into pauses.
- [A running whisper-server keeps `-nt` until restarted] → the PR body and the release notes tell the user to restart it.
- [VAD model missing on a machine that ran `just models` before] → `start-model-servers` names `just models`, which skips files already present and fetches only the new one.
- [Timestamps may split a word across segment lines] → `normalize_transcript` joins lines without a space, which already handles a split inside a word.
