# recognize-english-terms -> english-words-in-latin: misheard technical terms rule

The user decided (2026-10-18) that the refinement rule for technical terms misheard as Russian syllables
belongs to english-words-in-latin; recognize-english-terms only adds a default speech recognition prompt.

## Rule tested (appended before "Never translate:" in the current instruction)

The speech recognizer often mishears English technical terms (shell, tool, library, product names) spoken inside Russian speech and writes them as Cyrillic syllables or unrelated Russian words, split into pieces. When a run of odd Russian words or syllables sounds like a known technical term and fits the topic, replace it with that term in its usual Latin spelling (о май фиш -> oh-my-fish, пи эн пи эм -> pnpm, гит хаб экшнс -> GitHub Actions, вим плаг -> vim-plug).

Extra examples (after the 4 defaults):
- ну короче я поставил о май фиш и плагин вим плаг -> Ну, короче, я поставил oh-my-fish и плагин vim-plug.
- надо обновить гит хаб экшнс и пи эн пи эм -> Надо обновить GitHub Actions и pnpm.

## Results (temperature 0)
```
== Qwen2.5-1.5B (8080)
Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки. 
  -> Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки.
погляди был ли сохранен конфиг о май зэ эс эйч зэ эс эйч и пауэрлевел 10к настройки 
  -> Погляди был ли сохранен конфиг oh-my-fish на эс-эйч и пауэрлевел 10к настройки?
Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки 
  -> Погляди, был ли сохранен конфиг oh-my-zsh, zsh, и Powerlevel10k настройки.
поставь о май зш и тему пауэрлевел десять к 
  -> Поставь oh-my-zsh и тему powerlevel10k.
погляди был ли сохранен конфиг а май zsh zsh и по-ирлевелтинке на стройке 
  -> Погляди был ли сохранен конфиг а май zsh zsh и по-ирлевелтинке на стройке.
поставил о май зи эс эйч и тему пауэрлевел 10к 
  -> Поставил oh-my-zsh и тему Powerlevel 10k.
нужно обновить гит хаб экшнс и вс код 
  -> Нужно обновить GitHub Actions и вст код.
== Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf
Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки. 
  -> Погляди, был ли сохранён конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки.
погляди был ли сохранен конфиг о май зэ эс эйч зэ эс эйч и пауэрлевел 10к настройки 
  -> Погляди, был ли сохранён конфиг oh-my-zsh zsh и powerlevel10k настройки?
Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки 
  -> Погляди, был ли сохранён конфиг oh-my-zsh, zsh и Powerlevel10k настройки?
поставь о май зш и тему пауэрлевел десять к 
  -> Поставь oh-my-zsh и тему powerlevel10k.
погляди был ли сохранен конфиг а май zsh zsh и по-ирлевелтинке на стройке 
  -> Погляди, был ли сохранён конфиг а-май zsh zsh и по-ирлевелтинке на стройке?
поставил о май зи эс эйч и тему пауэрлевел 10к 
  -> Поставил oh-my-zsh и тему powerlevel10k.
нужно обновить гит хаб экшнс и вс код 
  -> Нужно обновить GitHub Actions и VS Code.
== google_gemma-3-12b-it-Q4_K_M.gguf
Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки. 
  -> Погляди, был ли сохранен конфиг. Ох, мой, за TSAzh, за TSAzh и Powerline 10K настройки.
погляди был ли сохранен конфиг о май зэ эс эйч зэ эс эйч и пауэрлевел 10к настройки 
  -> Погляди, был ли сохранен конфиг oh my ssh ssh и powerlevel 10k настройки.
Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки 
  -> Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки.
поставь о май зш и тему пауэрлевел десять к 
  -> Поставь oh-my-zsh и тему powerlevel ten k.
погляди был ли сохранен конфиг а май zsh zsh и по-ирлевелтинке на стройке 
  -> Погляди, был ли сохранен config oh-my-zsh, zsh и по-ирлевелтинке на стройке.
поставил о май зи эс эйч и тему пауэрлевел 10к 
  -> Поставил oh my ssh и тему Powerlevel10k.
нужно обновить гит хаб экшнс и вс код 
  -> Нужно обновить GitHub Actions и VS Code.
```

Summary: Qwen3-4B fixes every phonetic variant (о май зэ эс эйч -> oh-my-zsh, пауэрлевел 10к -> powerlevel10k, вс код -> VS Code);
1.5B hallucinates from the examples (oh-my-fish); gemma-12b writes "oh my ssh". Nobody restores the fully garbled
"Ох, мой, за ТСАж" — recognize-english-terms covers that with the recognition prompt.
Suggested eval cases (technical terms): the inputs above with expected oh-my-zsh / zsh / Powerlevel10k / VS Code.

## Temperature 0.3 / 0.7 (3 runs each)

No gain: Qwen3-4B gives the same answers at 0, 0.3 and 0.7; 1.5B gets worse at 0.7 (oh-my-fish, "o-mi-zes-es-ches-e"). Keep temperature 0.

## Rule v2 — written to the user's guidance (explicit IT domain, input is erroneous Russian dictation)

Important: the transcript is Russian dictation produced by a speech recognizer, and it contains recognition errors. The speaker is a software developer, so the dictation is about IT: programming, DevOps, shells, terminals, tools, libraries, services, cloud. English technical terms spoken inside the Russian speech usually come out wrong: written in Cyrillic, split into syllables, or replaced by Russian words that merely sound similar (о май фиш, пи эн пи эм, гит хаб экшнс, вим плаг). Your task: find every such fragment, work out which English IT term it sounds like, and write that term in English in its usual spelling (oh-my-fish, pnpm, GitHub Actions, vim-plug). Judge by sound and by the IT topic of the sentence. Leave ordinary Russian words that make sense as they are.

Same 2 extra examples, temperature 0. Results:
```
== Qwen2.5-1.5B (8080)
  -> Погляди, был ли сохранен конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки.
  -> Погляди был ли сохранен конфиг oh-my-zsh настройки и пауэрлевел 10к?
  -> Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки.
  -> Поставь oh-my-zsh и тему powerlevel ten.
  -> Погляди был ли сохранен конфиг а май zsh zsh и по-ирлевелтинке на стройке.
  -> Поставил oh-my-zsh и тему Powerlevel 10k.
  -> Нужно обновить GitHub Actions и вим код.
== Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf
  -> Погляди, был ли сохранён конфиг. Ох, мой, за ТСАж, за ТСАж и Powerline 10K настройки.
  -> Погляди, был ли сохранён конфиг oh-my-zsh zsh и powerlevel10k настройки?
  -> Погляди, был ли сохранён конфиг oh-my-zsh, zsh и Powerlevel10k настройки?
  -> Поставь oh-my-zsh и тему powerlevel10k.
  -> Погляди, был ли сохранён конфиг а-май zsh, zsh и по-ирлевелтинке на стройке?
  -> Поставил oh-my-zsh и тему powerlevel10k.
  -> Нужно обновить GitHub Actions и VS Code.
== google_gemma-3-12b-it-Q4_K_M.gguf
  -> Погляди, был ли сохранен конфиг. Ох, мой, за TSAZH, за TSAZH и Powerline 10K настройки.
  -> Погляди, был ли сохранен конфиг oh my SSH SSH и PowerLevel 10k настройки.
  -> Погляди, был ли сохранен конфиг oh-my-zsh, zsh и Powerlevel10k настройки.
  -> Поставь oh-my-fish и тему powerlevel ten k.
  -> Погляди, был ли сохранен config am zsh zsh и по-ирлевелтинке на стройке.
  -> Поставил oh my ssh и тему Powerlevel10k.
  -> Нужно обновить GitHub Actions и VS Code.
```

Qwen3-4B: same quality as v1 (all phonetic variants fixed, VS Code). 1.5B: better than v1 (no oh-my-fish on that line) but drops/garbles terms (вим код, powerlevel ten). gemma-12b: no better (oh-my-fish, SSH). Nobody fixes "Ох, мой, за ТСАж" or "по-ирлевелтинке".
