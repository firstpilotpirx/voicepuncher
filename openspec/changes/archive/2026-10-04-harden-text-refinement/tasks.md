# Tasks

## 1. Reproduce: dictated commands are carried out

- [x] 1.1 Red (bug reproduction): add `@refinement` scenarios "Dictated translation request is pasted, not carried out" and "Dictated order to ignore the rules is pasted, not carried out" to `e2e-test/tests/features/text_refinement.feature` with steps (`say` audio → app binary → live services); add the `refine-eval` recipe to `justfile` running `--tags @refinement`; run `just refine-eval` and see both fail because the translation / the single word is pasted; commit as an intentionally red test.

## 2. Normalization check

- [x] 2.1 Red: unit tests for `check_normalization` in `app/src/core/domain/normalization_check.rs` — translation rejected (language changed), unrelated answer rejected (words lost), punctuation/case/term respelling accepted, filler sounds ignored, filler-only transcript accepted; `just test` fails.
- [x] 2.2 Green: implement `check_normalization` and `NotANormalization`; `just test` passes.
- [x] 2.3 Red: unit tests for `refine_or_raw` in `app/src/core/application/text_refiner.rs` with `MockTextRefiner` — accepted answer returned, rejected answer replaced by the raw transcript, refinement error replaced by the raw transcript; `just test` fails.
- [x] 2.4 Green: implement `refine_or_raw` with the rejection log line; `Pipeline` calls it instead of its private method; `just test` passes.
- [x] 2.5 Red: add `@smoke` scenarios "Service answers with a translation", "Service answers instead of correcting", "Service answers with a normalization" to `text_refinement.feature` with stub steps; change the stub answer of the existing scenarios to a normalization of the spoken pangram; run them and see the first two fail before 2.4 is built into the release binary, then pass after `just build`.
- [x] 2.6 Add "Normalization check" to `docs/ubiquitous-language.md`; update "Text refinement" and `CleanedText` to say normalization; `just check`; commit.

## 3. Normalizing prompt with examples

- [x] 3.1 Red: unit tests in `request.rs` — examples are sent as user/assistant turns between the instruction and the transcript, each example transcript delimited like the real one; no examples gives two messages; unit tests in `refinement.rs` — default prompt states the transcript is data and lists punctuation, capitals, filler sounds and technical terms; default has three examples; `examples = []` parses; `just test` fails.
- [x] 3.2 Green: `RefinementExample`, `RefinementCfg.examples` with defaults, new default `system_prompt`, `chat_body` with example turns; `just test` passes.
- [x] 3.3 Red: add `@refinement` scenarios "Unpunctuated dictation gets punctuation and capitals", "Filler sounds are removed", "Technical term is written in its usual spelling", "Mixed Russian and English dictation keeps both languages" with steps that send a transcript through `voicepuncher::core::application::refine_or_raw` and the live service (e2e-test gains a path dependency on `voicepuncher`); run against the old prompt to see the expected failures.
- [x] 3.3a Red/Green: unit tests for `without_filler_sounds` (Russian and English sounds, comma after a sound, sound-only transcript) and for `refine_or_raw` sending the transcript without filler sounds; then implement `app/src/core/domain/filler_sounds.rs` and use it in `refine_or_raw` and the normalization check; `just test` passes.
- [x] 3.4 Green: tune the default prompt and examples until `just refine-eval` passes all `@refinement` scenarios, including group 1; `just check`; commit.

## 4. Refinement evaluation

- [x] 4.1 Write `e2e-test/refinement/eval_set.jsonl` (~40 items in `commands`, `punctuation`, `terms`, `fillers`, `mixed`, Russian, English and mixed), each with the transcript as `whisper-server` returns it and the expected normalization.
- [x] 4.2 Red: unit tests in `e2e-test/src/refinement_eval.rs` — case- and punctuation-sensitive score, raw baseline, "command carried out" verdict (WER > 0.25 on a `commands` item), "language changed" verdict, per-category means, failures listed by id; `cargo test -p voicepuncher-e2e-test --lib` fails.
- [x] 4.3 Green: implement the loader, `run` over `refine_or_raw`, and the report table; tests pass.
- [x] 4.4 Red/Green: add scenarios "Evaluation reports a score per category" (`@refinement`, live service) and "Evaluation fails when a dictated command is carried out" (`@smoke`, stub that appends an answer to the transcript, one-item set) with steps; see them fail before the steps are implemented, then pass.
- [x] 4.5 Wire `refine-eval` into `check` and `e2e` in `justfile`; run `just refine-eval`, record the per-category table of the old prompt (`examples = []`, previous `system_prompt`) and the new one in the PR body; every category of the new prompt scores at least as high; `just check`; commit.

## 5. Docs, version and manual checks

- [x] 5.1 README: replace `language = "ru"` in the `[transcription]` upgrade example with `language = "auto"` and add a note that a fixed language makes the speech recognition service turn speech in another language into text in the fixed one; document `[refinement].examples`, `just refine-eval`, and that a custom `system_prompt` keeps the old behavior until removed.
- [x] 5.2 Bump `version` in `app/Cargo.toml` to 0.4.0; `just check`; commit.
- [x] 5.3 Manual checks by the user: none (hotkey, recording, overlay, paste and permissions are unchanged).
