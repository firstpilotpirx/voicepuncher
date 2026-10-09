# Proposal

## Why

English technical terms inside Russian dictation come out garbled, and refinement does not repair them. In a real dictation the user said "Погляди, был ли сохранен конфиг: oh-my-zsh, zsh и Powerlevel10k настройки". The app pasted "Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки." The `text-refinement` spec already promises that technical terms come out in their usual Latin spelling, so this is a bug. The raw transcript looks worse than it used to, so speech recognition itself may have regressed.

## What Changes

- Find where the terms get lost. Compare the raw transcript with the refined text for this dictation and for similar ones, and check whether speech recognition regressed against an earlier version, model or decoding setup.
- Fix speech recognition so the raw transcript keeps English terms recognizable, if it turns out to be the cause.
- Fix refinement so terms that speech recognition misheard in Cyrillic or as other words ("Ох, мой, за ТСАж" → "oh-my-zsh", "Powerline 10K" → "Powerlevel10k") reach the paste in their usual Latin spelling.
- Add the reported dictation as a reproduction case to the refinement evaluation and to the tests.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-refinement`: add a scenario that reproduces the bug. Misheard English terms in a Russian dictation are restored to their usual Latin spelling.
- `speech-recognition`: if the cause is there, add a requirement that English terms spoken inside Russian speech come out recognizable in the raw transcript.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: English technical terms (tool, shell and library names) spoken inside Russian dictation, and the cause of the suspected recognition regression.
- Acceptance bar: the reported dictation is pasted with "oh-my-zsh", "zsh" and "Powerlevel10k" in Latin script and the rest in Russian. The refinement evaluation's technical-terms score does not drop.

## Out of scope

- A user-editable vocabulary or term list.
- Changing the default speech recognition or refinement model, unless the investigation shows the regression comes from the model.
- Languages other than Russian and English.

## Release

PATCH (bug fix), next patch version after the current one in `app/Cargo.toml` (0.6.0 → 0.6.1 at the time of writing).

## Impact

- Default refinement instruction and the refinement evaluation set.
- Possibly speech recognition settings (prompt, decoding parameters) in the speech recognition adapter.
- E2E and accuracy benchmarks: a new case with mixed Russian and English terms.
