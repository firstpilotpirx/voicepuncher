# Proposal

## Why

English technical terms inside Russian dictation come out garbled. The user said "Погляди, был ли сохранен конфиг: oh-my-zsh, zsh и Powerlevel10k настройки"; the app pasted "Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки."

The exploration (2026-10-18, synthesized speech, local services) found where the terms get lost:

- Speech recognition: without a prompt it writes "о мой ZSH, ZSH PowerLVL10 к настройке"; with a prompt naming the terms it writes "oh my zsh, zsh Powerlevel10…". No prompt is configured by default.
- Refinement cannot repair them: Qwen2.5-1.5B, Qwen3-4B and gemma-3-12b all leave "Ох, мой, за ТСАж" as it is or invent other words ("my SSH", "oh-my-fish"). A misheard term carries too little of the word to restore it from context.
- No regression of speech recognition was found.

## What Changes

- Speech recognition gets a default prompt that names common developer terms (shell, tool and service names), so they reach the raw transcript in their usual Latin spelling. A prompt the user configures replaces it; an explicitly empty prompt turns it off.
- The reported dictation becomes an E2E reproduction case for speech recognition, and its correct transcript a technical-terms case in the refinement evaluation, so refinement must keep such terms unchanged.
- The refinement instruction stays as it is: `english-words-in-latin` rewrites it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: common developer terms are recognized in their Latin spelling without any configuration.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: the default speech recognition prompt, the reproduction test, one refinement evaluation case.
- Acceptance bar: the reported sentence, synthesized, has "zsh" and "Powerlevel10k" in Latin script in the raw transcript; dictation WER on the 100 recordings of `just full-check` is not worse than without the prompt; the refinement evaluation's technical-terms score does not drop.

## Out of scope

- A user-editable term list beyond the existing prompt setting.
- Changing the speech recognition or refinement model, or the refinement instruction.
- Comparing with the unquantized recognition model.
- Languages other than Russian and English.

## Release

PATCH (bug fix): 0.15.3 → 0.15.4.

## Impact

- Default `[transcription] prompt` (`app/src/config/transcription.rs`), README and the setting's doc comment.
- `speech_recognition.feature` E2E scenarios, `e2e-test/refinement/eval_set.jsonl`.
