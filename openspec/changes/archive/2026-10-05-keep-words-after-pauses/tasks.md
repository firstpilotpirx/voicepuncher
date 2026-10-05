# Tasks

## 1. Measure dropped phrases

- [x] 1.1 Red: in `e2e-test/src/speech.rs`, add the unit tests `should_count_a_dropped_phrase` (spoken "a b c d e f g h", transcript "a b g h" → 4), `should_not_count_a_misheard_phrase` (same number of words in the gap → 0), `should_count_a_phrase_replaced_by_a_shorter_one` (8 spoken words in a gap filled by 2 → 6) and `should_count_words_dropped_at_the_end` (trailing spoken words missing → their count), with a stub `dropped_phrase_len` that returns 0; run `just test`, see them fail.
- [x] 1.2 Green: implement `dropped_phrase_len` by aligning the gaps of the longest common subsequence (design § Decisions 2); `just test` passes.
- [x] 1.3 `just check`; commit `test(e2e): measure the longest dropped phrase of a transcript`.

## 2. Keep the speech after a pause

- [x] 2.1 Red: in `app/tests/chunked_transcription_it.rs`, fail a case whose chunk transcripts drop a phrase of more than 5 spoken words (`MAX_DROPPED_PHRASE_WORDS = 5`) and print the dropped length per case (design § Decisions 3).
- [x] 2.2 Red: in `e2e-test/tests/features/speech_recognition.feature`, add `Rule: Speech after a pause is transcribed` with a `@seams` Scenario Outline: a recording file of Russian or English speech longer than the chunk threshold, with pauses between words or between sentences; the user transcribes it without refinement; then no run of more than five consecutive spoken words is missing from the printed text. Add the Then step to `e2e-test/tests/steps/speech_recognition.rs`.
- [x] 2.3 Run `just e2e-seams` against the service with today's flags (the running one on port 8081); see the sweep and the new scenarios fail on dropped phrases. Commit on its own as `test: reproduce phrases dropped after pauses (intentionally red)`.

- [x] 2.4 Green: remove `-nt` from `start-model-servers` in `justfile` (and fix its comment), from `STT_DECODING_FLAGS` in `e2e-test/scripts/bench.py` and from the manual start command in `README.md`; add a README note that a running whisper-server must be restarted to pick up new flags.
- [x] 2.4a Red: run the `@smoke` E2E against the service without `-nt`; see "Silence produces a lone \"you\"" fail on "Thank you." (design § Decisions 1a).
- [x] 2.4b Green: start whisper-server with `--vad -vm "$VOICEPUNCHER_VAD_MODEL" -vsd 2000 -vp 500`; `VOICEPUNCHER_VAD_MODEL` defaults to the newest `ggml-silero-*.bin` in `~/.voicepuncher/` or `~/.voicepuncher/models/`, `start-model-servers` requires it; `just models` downloads `ggml-silero-v5.1.2.bin`; the whisper model discovery skips `ggml-silero-*`; same flags and model in `bench.py`; README documents the model and flags.
- [x] 2.4c Red: in `app/tests/transcription_openai_it.rs`, add `should_return_no_transcript_when_service_hears_no_speech` (stub answers `{"text": ""}`, expects `None`) and `should_return_no_transcript_when_service_answers_only_whitespace` (`{"text": " \n"}`); the silence `@smoke` step also asserts the app exits without a failure; run `just test`, see it fail; commit on its own (intentionally red).
- [x] 2.4d Green: `OpenAiTranscriber::transcribe` returns `Ok(None)` for an empty or whitespace-only text (design § Decisions 1b); `just test` passes.
- [x] 2.5 Run `VOICEPUNCHER_STT_PORT=8092 just e2e-seams`; the sweep and every `@seams` scenario pass.
- [x] 2.6 Run `VOICEPUNCHER_STT_PORT=8092 just e2e-bench`; mean WER at most 0.070.
- [x] 2.7 `VOICEPUNCHER_STT_PORT=8092 just check`; commit `fix(speech-recognition): keep the speech after a pause`.

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.5.1; `VOICEPUNCHER_STT_PORT=8092 just check`; commit `chore: release 0.5.1`.
- [x] 3.2 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged); after the merge the user restarts the running whisper-server.
