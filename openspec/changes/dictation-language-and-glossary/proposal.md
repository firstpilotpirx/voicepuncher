## Why

Dictation runs Whisper large-v3 with `language = "auto"` and no vocabulary hint. English terms the user speaks inside Russian ("assign", "issue", "PR", "merge", "Claude Code") come out russified or garbled ("за сайне на меня", "шью", "опишки"). Paid cloud recognition is rejected, so the local recognizer must get better on the user's mixed speech.

## What Changes

- Dictation tells the recognizer the speech is Russian instead of guessing the language.
- The recognizer gets a glossary (prompt) of the user's terms, so English words come out in Latin spelling.
- English words must stay English: mixed Russian-English speech must not get worse than today.

Current stage: PoC. Target stage: PoC.
Scope: the dictation language setting, the Whisper prompt/glossary and where the user edits it, a before/after measurement on the user's own dictations.
Acceptance bar: on a set of the user's own dictations with English terms (from `~/.voicepuncher/history`), term errors drop and the overall word error rate is not worse; English terms stay in Latin spelling.
Out of scope: changing the model; paid or cloud recognition; noise suppression; chunk cutting.
Interface: none (config file only).

## Capabilities

### Modified Capabilities

- `dictation`: dictation language and a recognizer glossary of the user's terms.

## Impact

- Transcription adapter (request to whisper-server), configuration `[transcription]`, accuracy bench/baseline.

## Open points

- `language = "ru"` may make Whisper transliterate English words into Cyrillic — must be measured against `auto`; proposed: pick by the measurement, possibly `ru` + glossary, or `auto` + glossary.
- Where the glossary lives and who fills it — proposed: a list in the config, seeded with terms from the user's archive.
- Whisper prompts can make the model repeat or hallucinate the prompt on silence — check on short and silent clips.
- Interaction with the local LLM refinement prompt, which already writes terms in Latin — keep both or move terms into one list.
- Earlier note: about 1 in 6 large-v3 clips comes out lowercase without punctuation; a prompt with punctuation may fix it too.

## Release

SemVer: MINOR — dictation behavior changes (language and glossary).
Target version: 0.51.0.
