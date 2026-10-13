# Proposal

## Why

When the user dictates Russian with English words in it, speech recognition often writes those words in Cyrillic, and refinement leaves them there. In a real dictation the user said "automatic" and "dynamic"; the app pasted «Ну, лучше тогда автоматик, … В этом плане, наверное, автоматик лучше, чем даймэк.» The user expects «automatic» and «dynamic» in Latin script. «Даймэк» is worse than a transliteration: speech recognition misheard the word, and only the context ("automatic vs dynamic", microphone modes) tells what was said.

Refinement already writes technical terms and product names in Latin script (докер → Docker), but ordinary English words (automatic, dynamic, feedback, workaround) are not covered. A research run on 2026-10-09 (`research/README.md`) showed that the current default model, Qwen2.5-1.5B, cannot do this reliably even with an explicit rule and an example: it fixes 6 of 10 test phrases, never the user's phrase, and sometimes damages surrounding text. A larger local model with the same rule fixes 9 of 10, including the user's phrase, at a higher latency.

## What Changes

- Refinement writes an English word that speech recognition spelled in Cyrillic in its English Latin spelling (автоматик → automatic, фидбек → feedback, мануал → manual), and restores one that was misheard into a sound-alike non-word when the context makes it clear (даймэк → dynamic).
- Refinement keeps in Cyrillic every common Russian loanword (компьютер, менеджер, интернет, релиз, файл) and every Russified word with a Russian ending or suffix (деплоим, закомитить, пушнул).
- The default refinement model changes from Qwen2.5-1.5B-Instruct to a larger local model chosen by the research: google gemma-3-12b-it Q4_K_M (best quality, about 1.7 s on a long phrase) or Qwen3-4B-Instruct-2507 Q4_K_M (about 0.8 s, weaker on misheard words). The choice and its latency budget are settled at the start of the implementation (`design.md` § Open question).
- The refinement evaluation gains a category for English words in Cyrillic, with loanword controls, built from the research cases.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-refinement`: a requirement that English words written in Cyrillic come out in Latin script while Russian loanwords stay Cyrillic; the evaluation reports the new category; the default model changes.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: Russian dictation with English words that speech recognition spelled in Cyrillic or misheard into sound-alikes; the default refinement model and instruction; the refinement evaluation set.
- Acceptance bar: the user's dictation above is pasted with «automatic» and «dynamic» in Latin script and the rest unchanged; at least 8 of the 10 research cases pass, with all three loanword controls; no category of the refinement evaluation scores lower than with the current model; the refinement latency on a 180-character phrase stays within the budget the design settles.

## Out of scope

- Changing speech recognition (model, prompt, decoding): the research showed a recognition prompt does not change how accented English words are spelled.
- A user-editable vocabulary or term list.
- Languages other than Russian and English.
- Technical terms misheard into several Russian words (oh-my-zsh, Powerlevel10k): that is the filed change `recognize-english-terms`; the two changes share the refinement instruction and evaluation set, so whichever lands second merges its rule into the other's.

## Release

MINOR (behavior change: new refinement behavior and a new default model), the next minor version after the current one in `app/Cargo.toml` (0.9.0 → 0.10.0 at the time of writing).

## Impact

- Default refinement instruction and examples (`app/src/config/refinement.rs`).
- Default model download in `just models` and the README model instructions; users must download the new model (7.3 GB for gemma-3-12b, 2.5 GB for Qwen3-4B).
- `justfile` model auto-discovery picks the largest `.gguf`, so the new default wins once downloaded.
- Refinement evaluation set and the `@refinement` E2E scenarios.
- Research data: `research/` in this change (method, results, raw answers, benchmark script).
