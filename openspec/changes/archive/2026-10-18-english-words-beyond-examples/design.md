# Design

## Context

With the default instruction and seven examples (`app/src/config/refinement.rs`), Qwen3-4B-Instruct-2507 Q4_K_M leaves «шаффл» and «репит» in Cyrillic, and also «хай приорити». Measured on 2026-10-18 with a probe that sends the app's exact request (instruction, example turns, `<transcript>` wrapping, temperature 0) to an own `llama-server` on a free port, over 25 phrases: the bug dictation, new English words, loanwords that must stay, and the IT-term dictations of the `@refinement` scenarios.

| Variant | шаффл/репит | Regressions |
|---|---|---|
| Rule wording only | stays Cyrillic | none |
| Rule + example «включи режим лоу латенси и выключи эко мод» | fixed | «пауза» → pause, «лайк» → like, «груминг бэклога» → grooming backlog |
| Rule + example «открой плейлист поставь лайк треку и включи лууп» | fixed | «джава скрипт» → «Java Script» |
| Same example as the first turn | stays Cyrillic | none |
| Rule + example «открой плейлист поставь лайк треку и включи лууп и кросс фейд» | fixed | none |

## Decisions

1. **Rule wording plus one example; the example does the work.** The English-words paragraph of the instruction says the rule holds for every English word, also one never seen in Cyrillic (a Cyrillic word without a Russian ending that is no ordinary Russian word and reads as a fitting English word is English), lists case endings as Russian endings, names ordinary Russian words (пауза, кнопка) as staying, and lists more dictionary loanwords (чат, лайк, плейлист, трек). The rule alone changed nothing; an example alone over-converts. The new last example `открой плейлист поставь лайк треку и включи лууп и кросс фейд` → `Открой плейлист, поставь лайк треку и включи loop и crossfade.` shows, in one turn, loanwords kept (плейлист, лайк, трек), a new English word converted (loop) and a word dictated in two parts written whole (crossfade), which keeps «джава скрипт» → JavaScript. It goes last: as the first turn it lost its effect. Rejected: a vocabulary list (unbounded), adding the bug's words to the prompt (the test would then only show the model repeating its examples).
2. **No word from the scenarios in the prompt.** The new example and rule words share no English word or sentence structure with any `@refinement` dictation; «шаффл», «репит», «пауза»'s sentence and «скип» stay out of the prompt.
3. **An evaluation case guards over-conversion.** `e2e-test/refinement/eval_set.jsonl` gets `english-11`: «нажми скип а потом пауза» → «Нажми skip, а потом пауза.» with `keep_cyrillic: ["пауза"]`, so a later prompt that turns an ordinary Russian word into English fails the run, like a loanword does.
4. **Known limit.** «хай приорити» still stays Cyrillic; the acceptance bar names only shuffle and repeat, and the evaluation's `english` category shows the trend.

## Test levels

| Scenario | Level |
|---|---|
| English words the instruction never shows come out in Latin script | E2E `.feature`, `@refinement`, live service (new; reproduces the bug) |
| English word spelled in Cyrillic comes out in Latin script | E2E `.feature`, `@refinement`, live service (existing, unchanged) |
| Two English words in one sentence both come out in Latin script | E2E `.feature`, `@refinement`, live service (existing, unchanged) |
| Misheard English word is restored from context | E2E `.feature`, `@refinement`, live service (existing, unchanged) |
| Russian loanwords stay in Cyrillic | E2E `.feature`, `@refinement`, live service (existing, unchanged) |
| Russified English verbs stay in Cyrillic | E2E `.feature`, `@refinement`, live service (existing, unchanged) |

## Dependencies

Waits for `refinement-scenarios-unlike-prompt`: it rewrites the same `@refinement` scenarios and steps (`text_refinement.feature`, `normalization.rs`), and its new dictations must stay unlike the prompt this change rewords; after it merges, the new rule and example are measured again on its dictations.
