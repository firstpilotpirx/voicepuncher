# Proposal

## Why

When the user dictates Russian with English words in it, speech recognition often writes those words in Cyrillic, and refinement leaves them there. In a real dictation the user said "automatic" and "dynamic"; the app pasted «Ну, лучше тогда автоматик, … В этом плане, наверное, автоматик лучше, чем даймэк.» The user expects «automatic» and «dynamic» in Latin script. «Даймэк» is worse than a transliteration: speech recognition misheard the word, and only the context ("automatic vs dynamic", microphone modes) tells what was said.

Refinement already writes technical terms and product names in Latin script (докер → Docker), but ordinary English words (automatic, dynamic, feedback, workaround) are not covered. A research run on 2026-10-09 (`research/README.md`) showed that the current default model, Qwen2.5-1.5B, cannot do this reliably even with an explicit rule and an example: it fixes 6 of 10 test phrases, never the user's phrase, and sometimes damages surrounding text. A larger local model with the same rule fixes 9 of 10, including the user's phrase, at a higher latency.

## What Changes

- Refinement writes an English word that speech recognition spelled in Cyrillic in its English Latin spelling (автоматик → automatic, фидбек → feedback, мануал → manual), and restores one that was misheard into a sound-alike non-word when the context makes it clear (даймэк → dynamic).
- Refinement restores an English IT term that speech recognition split into Cyrillic syllables or sound-alike Russian words (о май зэ эс эйч → oh-my-zsh, пауэрлевел десять к → Powerlevel10k, вс код → VS Code). The instruction says outright that the input is Russian dictation by a software developer with recognition errors, that English terms in it come out as misspelled Russian, and that they almost always come from IT (programming, DevOps). The user moved this rule here from `recognize-english-terms` on 2026-10-18; measurements are in `research/misheard-terms.md`.
- Refinement keeps in Cyrillic every common Russian loanword (компьютер, менеджер, интернет, релиз, файл) and every Russified word with a Russian ending or suffix (деплоим, закомитить, пушнул).
- The default refinement model changes from Qwen2.5-1.5B-Instruct (1.0 GB) to Qwen3-4B-Instruct-2507 Q4_K_M (2.5 GB, about 0.8 s on a 180-character phrase). The user chose it over gemma-3-12b-it on 2026-10-18: gemma fixes the user's whole phrase but takes 1.7–2.6 s; Qwen3-4B restores «даймэк» → «dynamic» but leaves «автоматик» in that phrase.
- The refinement evaluation gains a category for English words in Cyrillic, with loanword controls, built from the research cases.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-refinement`: a requirement that English words written in Cyrillic come out in Latin script while Russian loanwords stay Cyrillic; a requirement that IT terms split into Cyrillic syllables are restored; the evaluation reports the new category; the default model changes.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: Russian dictation with English words that speech recognition spelled in Cyrillic or misheard into sound-alikes; the default refinement model and instruction; the refinement evaluation set.
- Acceptance bar: «о май зэ эс эйч», «пауэрлевел десять к» and «вс код» come out as oh-my-zsh, Powerlevel10k and VS Code; at least 8 of the 10 research cases pass, with all three loanword controls; in the user's dictation above, «даймэк» comes out as «dynamic» («автоматик» may stay, a known limit of the chosen model); no category of the refinement evaluation scores lower than with the current model; the refinement of a 180-character phrase takes at most 1.5 s.

## Out of scope

- Changing speech recognition (model, prompt, decoding): the research showed a recognition prompt does not change how accented English words are spelled.
- A user-editable vocabulary or term list.
- Languages other than Russian and English.
- Speech recognition changes for misheard terms: `recognize-english-terms` adds a default recognition prompt. A transcript garbled beyond its sound ("Ох, мой, за ТСАж") is not restored by refinement.

## Release

MINOR (behavior change: new refinement behavior and a new default model), the next minor version after the current one in `app/Cargo.toml` (0.15.3 → 0.16.0 at the time of writing).

## Impact

- Default refinement instruction and examples (`app/src/config/refinement.rs`).
- Default model download in `just models` and the README model instructions; users must download the new model (2.5 GB).
- `justfile` model auto-discovery picks the largest `.gguf`, so the new default wins over Qwen2.5-1.5B once downloaded; the refinement evaluation in `just check` gets about twice as slow.
- Refinement evaluation set and the `@refinement` E2E scenarios.
- Research data: `research/` in this change (method, results, raw answers, benchmark script).
