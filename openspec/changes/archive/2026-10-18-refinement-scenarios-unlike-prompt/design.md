## Context

Three `@refinement` scenarios of `text-refinement` dictate words the default instruction shows the model: «фидбек» and «даймэк» are named in the system prompt, and «поставь о май зш и тему пауэрлевел десять к» follows the oh-my-fish example of `DEFAULT_EXAMPLES` (`app/src/config/refinement.rs`). Their pass cannot tell a working rule from a repeated example.

## Goals / Non-Goals

**Goals:** each of the three scenarios dictates a phrase whose sentence structure, English words and IT terms appear in neither the system prompt nor `DEFAULT_EXAMPLES`, and keeps its meaning and checks.

**Non-Goals:** the prompt, its examples, the default model, other scenarios, the evaluation set.

## Decisions

| Scenario | New dictation | Checks | Why |
|---|---|---|---|
| English word spelled in Cyrillic | «добавь ретрай для этого запроса» | «retry»; «добавь», «запроса» kept | one whole English word with no Russian ending, not a common loanword; neither the word nor «добавь … для …» is in the prompt or examples |
| Misheard English word restored from context | «сервис пишет слишком мало логов нужен уровень дибаг чтобы найти ошибку» | «debug»; no «дибаг» | «дибаг» is a non-word (the Cyrillic spelling is «дебаг»), and log levels make clear the word is debug; the previous dictation also leaned on «автоматик», a prompt word |
| Shell framework and theme as syllables | «для баша есть фреймворк баш ит с темой пауэр лайн» | «bash-it», «powerline» (any case); «фреймворк», «темой» kept | a framework and theme the prompt never names, a sentence unlike «я поставил X и плагин Y» |

- The phrases were tried on Qwen3-4B-Instruct-2507 Q4_K_M with the default instruction and examples at temperature 0 before they were chosen; each came out as the checks expect. Rejected: «ретрай»-like phrases with «поставь» (the structure of the «режим мануал» example), «призто» for prezto (the model kept it in Cyrillic: a model gap, not this change's scope), «баш ит … тема у него …» (the model split it into «bash и тёмная тема»).
- The step texts and spec scenarios name the new words; the step functions keep their structure and helpers.

## Test levels

| Scenario | Level |
|---|---|
| English word spelled in Cyrillic comes out in Latin script | E2E (`@refinement`, live model) |
| Two English words in one sentence both come out in Latin script | E2E (`@refinement`, live model), unchanged |
| Misheard English word is restored from context | E2E (`@refinement`, live model) |
| Russian loanwords stay in Cyrillic | E2E (`@refinement`, live model), unchanged |
| Russified English verbs stay in Cyrillic | E2E (`@refinement`, live model), unchanged |
| Shell framework and theme spelled as syllables are restored | E2E (`@refinement`, live model) |
| Product names spelled as syllables are restored | E2E (`@refinement`, live model), unchanged |

## Risks / Trade-offs

- [`english-words-beyond-examples`, still unstarted, changes the same prompt, examples and feature file] → whichever lands second merges the other and checks that the three dictations still share no word with the prompt or examples.
- [A later prompt change may name one of the new words] → the requirement now says the dictation differs from every word the instruction names, so such a change must pick new phrases too.
- [Model output varies between llama.cpp builds] → the checks look for the restored terms only, ignoring case, as before.
