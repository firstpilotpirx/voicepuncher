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

1. **Instruction rule plus example.** Add the rule from `research/README.md` § Method (prompt B) to the default instruction, and the example `короче поставь режим мануал а не автоматик` → `Короче, поставь режим manual, а не automatic.`. Reword "every English word exactly as dictated" so it covers English words already in Latin script only. The rule alone was ignored by 1.5B; the example is what makes smaller models follow it.
2. **Default model.** gemma-3-12b-it Q4_K_M is the recommended default: the only model that fixed the user's phrase, best eval similarity. Alternative: Qwen3-4B-Instruct-2507 Q4_K_M, when the latency budget is under 1 s. Qwen2.5-7B is dropped: slower than Qwen3-4B and worse on English words. A Qwen3-30B-A3B MoE (18.6 GB, 3B active) was not measured; it may match gemma at Qwen3-4B speed and is worth one benchmark run before deciding.
3. **Evaluation category `english_words`.** Add the 10 research cases (7 to convert, 3 loanword controls) to `e2e-test/refinement/eval_set.jsonl`; a control that comes out in Latin script counts as a failure, like a changed language today.
4. **Check the gemma chat template.** gemma has no system role; llama-server folds the system message into the first user turn. The research ran through llama-server's default template and worked; confirm the `<transcript>` data boundary still holds with the `commands` category (it scored 0.99).

## Open question

Latency budget for refinement on a long phrase: about 0.8 s (Qwen3-4B, user's phrase not fixed) or about 1.7–2.6 s (gemma-3-12b, fixed). Settle it with the user at the start of the implementation; benchmark Qwen3-30B-A3B first if a middle ground is wanted.

## Test levels

| Scenario (to be written in specs) | Level |
|---|---|
| English word in Cyrillic comes out in Latin script | E2E `.feature`, `@refinement`, live service |
| Misheard English word is restored from context (the user's phrase) | E2E `.feature`, `@refinement`, live service |
| Russian loanwords stay in Cyrillic | E2E `.feature`, `@refinement`, live service |
| Evaluation reports the English words category | E2E `.feature`, `@refinement` |
| Default instruction carries the rule and the example | unit test in `app/src/config/refinement.rs` |

## Risks / Trade-offs

- **Latency.** gemma-3-12b is about 4× slower than 1.5B; long dictations scale with output length. Mitigation: measure stop→paste in the manual run; fall back to Qwen3-4B if it feels slow.
- **Disk and memory.** 7.3 GB on disk and about 9 GB of RAM while running; fine on the M1 Max with 64 GB, heavy on a 16 GB machine.
- **Over-conversion.** The model may move established loanwords to Latin script. Mitigation: the loanword controls in the evaluation fail the run.
- **Fillers.** gemma with prompt B scored lower on fillers (0.91 → 0.87); check the fillers category before switching.
- **Overlap with `recognize-english-terms`.** Both edit the instruction and the evaluation set; the second to land rebases its rule onto the first.
