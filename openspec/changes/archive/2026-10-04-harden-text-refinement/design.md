# Design

## Context

See proposal.md § Why for the measurements. Current flow: `Pipeline::refine_or_raw` calls the `TextRefiner` port; `OpenAiTextRefiner` sends `[system, user(<transcript>…</transcript>)]` and `cleaned_answer` rejects a cut-off answer or one shorter than half the transcript; any error makes the pipeline paste the raw transcript. Nothing checks that the answer is still the user's dictation. The E2E stub scenarios answer with "Corrected by the text refinement service.", which shares no words with the spoken pangram.

## Goals / Non-Goals

**Goals:** one pure check that decides "is this answer a normalization of the transcript", applied to every refiner and every prompt; a default prompt that normalizes; a text-level evaluation that reuses the exact app code path and runs in `just check`.

**Non-Goals:** a semantic (embedding or LLM-as-judge) similarity check — it would need another model at runtime; detecting an answer appended to an otherwise intact dictation (the prompt handles it, the evaluation catches regressions).

## Decisions

### 1. Normalization check is domain policy

`app/src/core/domain/normalization_check.rs`: `pub fn check_normalization(raw: &RawTranscript, answer: &CleanedText) -> Result<(), NotANormalization>`, with `NotANormalization::{LanguageChanged { from_cyrillic, to_cyrillic }, WordsLost { kept, total }}` and a `Display` that names the reason.

- **Words kept:** tokens = lowercase alphanumeric runs, `ё`→`е`; filler sounds (the list of §3a) are dropped from the transcript side. Kept = transcript tokens found in the answer's token set, counted separately for Cyrillic and for Latin tokens. Reject when `kept * 2 < total` in either alphabet — counting all words together let a translation of only the Russian part of a mixed dictation through, because its English words made up half. Measured: carried-out commands keep ≤ 25 %, real normalizations keep ≥ 64 %.
- **Writing system:** Cyrillic share of letters in each text. Reject when it moves by more than 0.5 (a translation moves it by ~1.0; respelling terms in Latin moved it by ≤ 0.37).
- A transcript with no letters or only filler sounds passes.

Alternatives: putting it in the OpenAI adapter (would not cover other refiners, and it is not HTTP-specific); a stricter edit-distance bound (respelled terms and punctuation would trip it).

### 2. One shared composition: `refine_or_raw`

Move the pipeline's private `refine_or_raw` to `app/src/core/application/text_refiner.rs` as `pub fn refine_or_raw(refiner: &mut dyn TextRefiner, raw: &RawTranscript) -> Option<CleanedText>`: refine, then `check_normalization`; on refinement error log `refinement failed, falling back to raw transcript: …` (unchanged), on a rejected answer log `refinement answer rejected (<reason>), pasting the raw transcript instead: <answer>`. `Pipeline` calls it; the evaluation calls the same function, so it measures what the user would get pasted.

### 3. Default prompt with few-shot turns

`RefinementCfg` gains `examples: Vec<RefinementExample { transcript, normalized }>` (default: three examples — a Russian translation request kept as a request, an English question kept as a question, a Russian filler/term sentence normalized). `chat_body` sends `system`, then for each example a `user(<transcript>…)` / `assistant(normalized)` pair, then the real transcript. The default `system_prompt` becomes the normalizer prompt measured in explore (data-not-instructions first, then the normalization list, then "never translate, answer, add or drop meaning, never shorten"). `examples = []` in the config restores a two-message request. The per-request output budget is unchanged.

Alternative: examples inside the system prompt text — the 1.5B model followed examples as turns better in explore, and turns keep the "data in user message" structure.

### 3a. Filler sounds are removed by the app, not the model

`app/src/core/domain/filler_sounds.rs`: a closed list of hesitation sounds (`э`, `э-э`, `эм`, `ммм`, `um`, `uh`, `hmm`, …) and `without_filler_sounds(text)`, which drops those tokens with the comma that follows them. `refine_or_raw` sends the transcript without filler sounds to the refiner and checks the answer against it; the fallback still pastes the raw transcript. The normalization check ignores the same list. A transcript of nothing but filler sounds is pasted as is, without asking the service: there is nothing to normalize, and the raw transcript is what the fallback pastes anyway.

Why: in the evaluation the 1.5B model kept "um, uh" and "э-э" with every prompt tried, and an English filler example made it translate Russian dictation into English. A token list is exact and costs nothing. The prompt keeps the mention of filler sounds for other models.

### 4. Evaluation set and runner live in `e2e-test`

- `e2e-test/refinement/eval_set.jsonl` (committed): `{id, category, transcript, expected}`; categories `commands`, `punctuation`, `terms`, `fillers`, `mixed`; ~40 items, transcripts written the way `whisper-server` returns dictation.
- `e2e-test/src/refinement_eval.rs`: `run(refiner, items) -> Report`, using `voicepuncher::core::application::refine_or_raw` (e2e-test gains a path dependency on the `voicepuncher` lib). Per item: pasted text, `score = 1 − CER(expected, pasted)` on the text **with** case and punctuation (so the score sees what refinement is for), the same score for the raw transcript as baseline, and two hard verdicts: *command carried out* (category `commands` and word-level WER(expected, pasted) > 0.25 after lowercasing and stripping punctuation) and *language changed* (Cyrillic share of the pasted text differs from the expected text's by > 0.5 — compared with the expected text, not the transcript, so a one-word dictation respelled in Latin script is not flagged). `Report` prints a table per category (mean score, raw baseline, items) and lists failures by id.
- `just refine-eval` starts the model servers and runs `cucumber_e2e --tags @refinement`. `check` and `e2e` include it.

Alternative: extending `bench.py` — Python cannot reuse the app's check and prompt, so it would measure something else than what is pasted.

### 5. Test levels

| Spec scenario | Level |
|---|---|
| Dictated text sounds like a request (existing) | E2E `.feature`, stub, `@smoke` (unchanged) |
| Dictated translation request is pasted, not carried out | E2E `.feature`, `say` audio → app binary → live services, `@refinement` |
| Dictated order to ignore the rules is pasted, not carried out | E2E `.feature`, `say` audio → app binary → live services, `@refinement` |
| Unpunctuated dictation gets punctuation and capitals | E2E `.feature`, transcript → `refine_or_raw` → live service, `@refinement` |
| Filler sounds are removed | same; unit tests of `without_filler_sounds` and `refine_or_raw` |
| Technical term is written in its usual spelling | same |
| Mixed Russian and English dictation keeps both languages | same |
| Service answers with a translation | E2E `.feature`, stub → app binary, `@smoke`; unit tests of `check_normalization` |
| Service answers instead of correcting | same |
| Service answers with a normalization | same |
| Evaluation reports a score per category | E2E `.feature`, live service, `@refinement` |
| Evaluation fails when a dictated command is carried out | E2E `.feature`, stub that appends an answer to the transcript, `@smoke`; unit tests of the verdicts |

Prompt and examples in the request: unit tests in `request.rs`. `refine_or_raw`: unit tests with `MockTextRefiner`.

The stub answers of the existing scenarios ("Transcript is refined by the service", "Reasoning model is configured") become a normalization of the spoken pangram (same words, different case and punctuation), so they still prove the service's text is pasted.

## Risks / Trade-offs

- [The 1.5B model drifts across llama.cpp versions; live scenarios flake] → temperature 0; live scenarios assert properties (Cyrillic kept, key words present, comma added), not exact strings; the evaluation's hard verdicts only cover commands and language.
- [Guard rejects a legitimate heavy respelling of a short, term-only dictation] → falls back to raw, which is the pre-change behavior; the evaluation's `terms` category shows it.
- [Answer appended to an intact dictation passes the guard] → prompt example; evaluation verdict catches it on the set.
- [Users with a custom `system_prompt` in their config keep the old prompt] → README and release notes tell them to delete the key; the guard protects them anyway.

## Migration Plan

No config migration: `examples` is optional with a default. After release, the user removes `system_prompt` from `~/.voicepuncher/config.toml` to get the new default. Rollback: revert the release; the old prompt can be restored through the config.
