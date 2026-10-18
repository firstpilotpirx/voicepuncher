# Design

## Context

`[transcription] prompt` defaults to an empty string, so dictation is recognized without a prompt. Since `restore-recognition-prompt` a non-empty prompt reaches whisper-server (`carry_initial_prompt`, `max_context` 224) and steers every 30 s window except a last piece under 5 s. README § Writing `[transcription] prompt` records that large-v3-turbo ignores a bare list of English terms in Russian speech and follows a sentence that uses them.

## Decisions

1. **The default prompt is a Russian sentence that uses the terms.** Measured on 2026-10-18 with synthesized speech (Milena voice) on the shared dictation service:

   | Prompt | "о-май-зи-эс-эйч, зи-эс-эйч и пауэрлевел-тен-кей" | "oh my zsh, zsh и Powerlevel10k" |
   |---|---|---|
   | none | а май ZSH, ZSH и по-ирлевелтинке | о мой ZSH, ZSH PowerLVL10 к |
   | list of the terms | о-май-зсh, зсh, Powerlevel10ки | oh-my-zsh, zsh, Powerlevel10 к |
   | sentence using the terms | а май zsh, zsh и Powerlevel10k | oh-my-zsh, zsh Powerlevel10 к |

   The sentence: "В терминале у меня zsh с oh-my-zsh и темой Powerlevel10k, пакеты ставлю через Homebrew и npm. Код храню в Git на GitHub, собираю в Docker, деплою в Kubernetes, пишу в VS Code." With it, English speech, plain Russian speech and 8 s of silence came out exactly as without a prompt. Kept short (about 60 tokens): whisper keeps only the last 223 tokens of prompt plus earlier text, and a long prompt leaves less room for the text decoded in earlier windows.

2. **The default lives in the config default, not in the adapter.** `TranscriptionCfg::default_prompt()` returns the sentence and serde uses it when the key is absent; an explicit `prompt = ""` stays empty and sends no prompt, as today. The request code does not change, so the existing request-field tests keep holding.

3. **The refinement instruction does not change here.** The user moved the rule for misheard terms to `english-words-in-latin`; this change adds only an evaluation case with the correctly recognized sentence, so refinement must keep its terms as they are.

## Risks / Trade-offs

- **WER on real speech.** A prompt can steer common words toward the prompt's spelling ("конфиг" → "config" once in the measurements). `just full-check` runs the 100 recordings because `app/src/config/*` is on the recognition path; if WER is worse than on `main`, the change stops and reports it.
- **Echo on silence.** Whisper may write prompt text for unclear audio. Measured: silence gave no text; the "No speech yields no text" scenarios cover it.
- **Short dictations.** A dictation under 5 s is decoded without the prompt (whisper.cpp), so a short phrase with a term may still come out in Cyrillic; the spec already states this exception.
- **Users with their own prompt** keep it and get nothing new; documented in README.

## Test levels

| Scenario | Level |
|---|---|
| Shell terms inside Russian dictation come out in Latin script | E2E `.feature` (`speech_recognition.feature`, `@smoke`), synthesized sentence, no config |
| The user's own prompt replaces the default one | unit test in `app/src/config/transcription.rs` (a configured prompt is kept as written) |
| An empty prompt turns the prompt off | unit test in `app/src/config/transcription.rs` (`prompt = ""` stays empty) plus the existing request test that an empty prompt sends no prompt field |
| Correct technical terms stay unchanged in refinement | refinement evaluation case (`e2e-test/refinement/eval_set.jsonl`, technical terms) |
