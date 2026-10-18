# Design

## Context

Refinement sends the transcript to a local `llama-server` with a system instruction and four example turns (`app/src/config/refinement.rs`). The instruction already asks for technical terms in Latin script; it says nothing about ordinary English words, and it explicitly keeps English words "exactly as dictated", which a model may read as "keep the Cyrillic spelling".

Full research data: `research/README.md` (method, per-model table, raw answers), `research/results.json`, `research/bench_models.py` (rerunnable: `python3 -I bench_models.py <eval_set.jsonl> <out.json> <model.gguf>...`, uses port 8090).

Summary of the research (prompt B = current instruction + the new rule + one example):

| Model | Size | English cases | Eval similarity | Latency, 180-char phrase |
|---|---|---|---|---|
| Qwen2.5-1.5B-Instruct (current) | 1.0 GB | 6/10 | 0.874 | 0.39 s |
| Qwen3-4B-Instruct-2507 | 2.5 GB | 8/10 | 0.942 | 0.82 s |
| Qwen2.5-7B-Instruct | 4.7 GB | 5/10 | 0.947 | 1.22 s |
| gemma-3-12b-it | 7.3 GB | 9/10 (user's phrase fixed) | 0.971 | 1.72 s (max 2.63 s) |

## Goals / Non-Goals

**Goals:**

- English words in Cyrillic come out in Latin script; misheard sound-alikes are restored from context.
- Russian loanwords and Russified slang stay Cyrillic.
- The refinement evaluation measures both, so a model or prompt change shows its effect.

**Non-Goals:**

- Speech recognition changes.
- A vocabulary list.

## Decisions

1. **Instruction rule plus example.** Add the rule from `research/README.md` § Method (prompt B) to the default instruction, before "Never translate:", and a fifth example `короче поставь режим мануал а не автоматик` → `Короче, поставь режим manual, а не automatic.`. Reword "every English word exactly as dictated" to "every English word already in Latin script exactly as dictated", so it no longer tells the model to keep a Cyrillic spelling. The rule alone was ignored by small models; the example is what makes them follow it.
2. **Default model: Qwen3-4B-Instruct-2507 Q4_K_M.** The user chose it on 2026-10-18 over gemma-3-12b-it: 8/10 English cases, eval similarity 0.942 (1.5B: 0.874), 0.82 s on the 180-character phrase instead of 1.7–2.6 s. It restores «даймэк» → «dynamic» but leaves «автоматик» in the user's phrase; accepted. Rejected: gemma-3-12b (latency), Qwen2.5-7B (slower and worse), keeping 1.5B (6/10), benchmarking Qwen3-30B-A3B (18.6 GB download for a guess). Qwen3-4B-2507 is the non-thinking variant; the existing reasoning-block removal covers it anyway.
3. **Model found by file name.** The `justfile` finds the refinement model by its file name `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/`, like the Whisper models, instead of taking the largest `*.gguf`; `VOICEPUNCHER_LLM_MODEL` still overrides it, and a missing file fails with "Run 'just models' or set it in .env". `just models` downloads it from `bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF`, pinned to one revision with its sha256, and no longer downloads Qwen2.5-1.5B. Rejected: keeping "largest wins" — any larger research model left on disk would silently become the default and change what the checks measure.
4. **Evaluation category `english`.** Add the 10 research cases to `e2e-test/refinement/eval_set.jsonl` under category `english`: 7 to convert, 3 loanword controls. An item may list `keep_cyrillic` words; a listed word missing from the pasted text (compared case-insensitively, by its stem as written in the list) is a new verdict `LoanwordLeftCyrillic` that fails the run, like a changed language. Case 6 (стендап) expects the Cyrillic spelling, since it is an established loanword. Rejected: a separate pass/fail list per item for the converted words — the character score already measures them, and the category must beat its raw score.
5. **Latency budget 1.5 s on a 180-character phrase**, measured once with the new model in stage 5 and written in the PR; not a test, since it depends on the machine and on what else runs on the GPU.

## Verification

The shared text refinement service on port 8080 may still run the old model, and the checks reuse a service that answers on its port. This branch's checks run with `VOICEPUNCHER_LLM_PORT` set to a free port and `VOICEPUNCHER_LLM_MODEL` pointing at the downloaded Qwen3-4B, so they start their own service with the new model. After the merge the user restarts the shared service (`just stop-model-servers llama-server && just start-model-servers llama-server`) to pick up the new model.

## Test levels

| Scenario | Level |
|---|---|
| English word spelled in Cyrillic comes out in Latin script | E2E `.feature`, `@refinement`, live service |
| Two English words in one sentence both come out in Latin script | E2E `.feature`, `@refinement`, live service |
| Misheard English word is restored from context | E2E `.feature`, `@refinement`, live service |
| Russian loanwords stay in Cyrillic | E2E `.feature`, `@refinement`, live service |
| Russified English verbs stay in Cyrillic | E2E `.feature`, `@refinement`, live service |
| Evaluation reports a score per category | E2E `.feature`, `@refinement`, live service (existing, `english` added to the categories) |
| Evaluation fails when a dictated command is carried out | E2E `.feature`, `@smoke`, refinement service stub (existing, unchanged) |
| Word ending fixes improve on the raw transcript | E2E `.feature`, `@refinement`, live service (existing, unchanged) |
| English word fixes improve on the raw transcript | E2E `.feature`, `@refinement`, live service |
| Evaluation fails when a loanword is respelled in Latin script | E2E `.feature`, `@smoke`, refinement service stub; unit tests in `e2e-test/src/refinement_eval.rs` |
| Default instruction carries the rule and the example (no spec scenario) | unit test in `app/src/config/refinement.rs` |

## Risks / Trade-offs

- **The user's phrase stays half fixed.** Qwen3-4B leaves «автоматик» there; the evaluation keeps that case, so a later model or prompt change shows when it starts to pass.
- **Latency.** About twice the 1.5B latency (0.82 s vs 0.39 s on a long phrase); the evaluation stage of `just check` slows by about as much.
- **Disk and memory.** 2.5 GB on disk instead of 1 GB; fine on 16 GB machines.
- **Over-conversion.** The model may move an established loanword to Latin script; the loanword controls fail the evaluation.
- **Overlap with `recognize-english-terms`.** Both edit the default instruction and the evaluation set; the second to land merges its rule into the first's.
