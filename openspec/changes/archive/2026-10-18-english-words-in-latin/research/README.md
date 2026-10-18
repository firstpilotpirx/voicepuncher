# Research: refinement models and English words in Cyrillic

Run on 2026-10-09 on an M1 Max with 64 GB, llama.cpp `llama-server` (`-c 4096 -np 1`, Metal), Q4_K_M quantizations from huggingface.co/bartowski, temperature 0, the app's request shape (system prompt, four example turns, transcript in `<transcript>` tags, output budget of about one token per two characters). Script: `bench_models.py`; raw answers: `results.json`; current prompt: `prompt_current.txt`.

## Method

- **Eval similarity** — the 49 items of `e2e-test/refinement/eval_set.jsonl`; character similarity (difflib ratio) of the answer to the expected normalization, averaged per category and overall. A rough proxy, not the `just refine-eval` judge.
- **English cases** — 10 transcripts: 7 hold English words that speech recognition wrote in Cyrillic (one misheard into a non-word), 3 are controls with Russian loanwords and Russified slang that must stay Cyrillic. A case passes when every required word appears and no forbidden one does.
- **Latency** — wall time of one request after a warm-up; median and maximum over all 59 requests, plus the user's phrase (the longest English case, 180 characters).
- **Prompt A** (`A_current`) — today's default instruction and four examples.
- **Prompt B** (`B_rule`) — A plus the rule below inserted before "Never translate:", plus a fifth example.

Rule added in B:

> English words: when the recognizer wrote a whole English word in Cyrillic letters (автоматик -> automatic, дайнемик -> dynamic, фидбек -> feedback), write it in English, and fix it when it was misheard into a non-word that only sounds like it (даймэк -> dynamic). Leave in Cyrillic every word with a Russian ending or suffix (деплоим, закомитить, пушнул) and every common Russian loanword (компьютер, менеджер, интернет, релиз, файл).

Example added in B: `короче поставь режим мануал а не автоматик` → `Короче, поставь режим manual, а не automatic.`

## English cases

| # | Transcript | Must contain | Must not contain |
|---|---|---|---|
| 1 | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем даймэк. (the user's real dictation; said "automatic" and "dynamic") | automatic, dynamic | автоматик, даймэк |
| 2 | у этого микрофона есть режим дайнемик и режим кардиоид | dynamic, cardioid | дайнемик, кардиоид |
| 3 | дай мне фидбек по этому пулл реквесту | feedback | фидбек |
| 4 | включи в настройках опцию сайлент мод | silent | сайлент |
| 5 | это был такой воркараунд на время | workaround | воркараунд |
| 6 | у нас будет стендап в десять утра | stand | стендап |
| 7 | поставь режим мануал а не автоматик | manual, automatic | мануал, автоматик |
| 8 | менеджер попросил перезагрузить компьютер и проверить интернет | менеджер, компьютер, интернет | — |
| 9 | мы деплоим релиз завтра а потом закомитим фикс | деплои, релиз, закомит | — |
| 10 | скинь мне файл в чат и поставь лайк | файл, чат, лайк | — |

Case 6 is debatable: «стендап» is an established Russian loanword, so leaving it Cyrillic is acceptable.

## Results

| Model | Prompt | English cases | Eval similarity | commands | punctuation | terms | fillers | mixed | endings | Median latency | Max latency | User phrase latency |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Qwen2.5-1.5B-Instruct-Q4_K_M | A_current | 4/10 | 0.847 | 0.67 | 0.95 | 0.87 | 0.90 | 0.87 | 0.95 | 0.15 s | 0.40 s | 0.40 s |
| Qwen2.5-1.5B-Instruct-Q4_K_M | B_rule | 6/10 | 0.874 | 0.72 | 0.96 | 0.90 | 0.89 | 0.89 | 0.97 | 0.14 s | 0.39 s | 0.39 s |
| Qwen_Qwen3-4B-Instruct-2507-Q4_K_M | A_current | 4/10 | 0.933 | 0.86 | 0.97 | 0.96 | 0.91 | 0.96 | 1.00 | 0.28 s | 0.77 s | 0.77 s |
| Qwen_Qwen3-4B-Instruct-2507-Q4_K_M | B_rule | 8/10 | 0.942 | 0.86 | 0.97 | 0.96 | 0.92 | 0.99 | 1.00 | 0.27 s | 0.82 s | 0.82 s |
| Qwen2.5-7B-Instruct-Q4_K_M | A_current | 3/10 | 0.920 | 0.85 | 0.97 | 0.93 | 0.88 | 0.96 | 0.98 | 0.42 s | 1.33 s | 1.33 s |
| Qwen2.5-7B-Instruct-Q4_K_M | B_rule | 5/10 | 0.947 | 0.91 | 0.97 | 0.95 | 0.92 | 0.97 | 0.98 | 0.41 s | 1.26 s | 1.22 s |
| google_gemma-3-12b-it-Q4_K_M | A_current | 6/10 | 0.977 | 0.99 | 0.98 | 0.98 | 0.91 | 0.98 | 0.99 | 0.71 s | 1.96 s | 1.96 s |
| google_gemma-3-12b-it-Q4_K_M | B_rule | 9/10 | 0.971 | 0.99 | 0.96 | 0.98 | 0.87 | 0.98 | 1.00 | 0.70 s | 2.63 s | 1.72 s |

Findings:

- Only gemma-3-12b with prompt B fixes the user's phrase in full ("…лучше тогда automatic… automatic лучше, чем dynamic."); it misses only case 6, which is acceptable. No model in any variant broke a loanword control except Qwen3-4B with prompt A, which respelled «закомитим» as «закоммитим» (still Cyrillic, the check is strict).
- Prompt B helps every model and does not lower eval similarity, except gemma (0.977 → 0.971, fillers 0.91 → 0.87).
- Qwen3-4B-2507 with prompt B is the speed/quality middle ground: 8/10, eval 0.942, about twice as slow as 1.5B, but it leaves the user's phrase unchanged.
- Qwen2.5-7B is slower than Qwen3-4B and worse on English words; not a candidate.
- gemma-3-12b costs about 4× the latency of 1.5B (1.7 s on the user's phrase, 2.6 s worst case) and 7.3 GB instead of 1 GB on disk.

Earlier checks in the same investigation (2026-10-09, Qwen2.5-1.5B):

- The rule alone without an example was ignored by 1.5B; with an example it worked unreliably and sometimes damaged other text («режим мануал» → «режим Manu», dropped «а не мануал»).
- A speech recognition prompt (whisper initial prompt) did not help: an English word pronounced the English way already comes out in Latin script; one pronounced with a Russian accent comes out in Cyrillic with or without the prompt.

## Raw answers to the English cases


#### Qwen2.5-1.5B-Instruct-Q4_K_M — A_current

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем даймэк. |
| fail | У этого микрофона есть режим дайне́мик и режим кардио́ид. |
| fail | Дайте мне фидбек по этому pull request. |
| pass | Включи в настройках опцию silent mode. |
| fail | Это был такой ворк-раунд на время. |
| fail | У нас будет стендап в десять утра. |
| fail | Поставь режим "мануал" а не "автоматик". |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра и закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### Qwen2.5-1.5B-Instruct-Q4_K_M — B_rule

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем даймэк. |
| pass | У этого микрофона есть режим dynamic и режим cardioid. |
| fail | Дайте мне фидбек по этому pull request. |
| pass | Включи в настройках опцию silent mod. |
| fail | Это был такой ворк-раунд на время. |
| fail | У нас будет стендап в 10 утра. |
| pass | Поставь режим manual, а не automatic. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра и закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### Qwen_Qwen3-4B-Instruct-2507-Q4_K_M — A_current

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем Daimak. |
| fail | У этого микрофона есть режим динамик и режим кардиоид. |
| fail | Дай мне фидбек по этому пулл-реквесту. |
| pass | Включи в настройках опцию silent mode. |
| pass | Это был такой workaround на время. |
| fail | У нас будет стендап в десять утра. |
| fail | Поставь режим мануал, а не автоматик. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| fail | Мы деплоим релиз завтра, а потом закоммитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### Qwen_Qwen3-4B-Instruct-2507-Q4_K_M — B_rule

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем dynamic. |
| pass | У этого микрофона есть режим dynamic и режим cardioid. |
| pass | Дай мне feedback по этому pull requestу. |
| pass | Включи в настройках опцию silent mode. |
| pass | Это был такой workaround на время. |
| fail | У нас будет стендап в десять утра. |
| pass | Поставь режим manual, а не automatic. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра, а потом закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### Qwen2.5-7B-Instruct-Q4_K_M — A_current

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем Даймэк. |
| fail | У этого микрофона есть режим динамик и режим кардиоид. |
| fail | Дай мне фидбек по этому пулл-:requestу. |
| fail | Включи в настройках опцию Сайлент Мод. |
| fail | Это был такой воркараунд на время. |
| fail | У нас будет стендап в десять утра. |
| fail | Поставь режим мануал, а не автоматик. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра, а потом закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### Qwen2.5-7B-Instruct-Q4_K_M — B_rule

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем dynamic. |
| fail | У этого микрофона есть режим dynamic и режим кардиоид. |
| fail | Дай мне фидбек по этому пулл-реквесту. |
| pass | Включи в настройках опцию Silent Mode. |
| fail | Это был такой воркаранд на время. |
| fail | У нас будет стендап в десять утра. |
| pass | Поставь режим manual а не automatic. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра, а потом закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### google_gemma-3-12b-it-Q4_K_M — A_current

| | Output |
|---|---|
| fail | Ну, лучше тогда автоматик, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, автоматик лучше, чем Daemec. |
| fail | У этого микрофона есть режим динамик и режим кардиоид. |
| fail | Дай мне фидбек по этому pull request. |
| pass | Включи в настройках опцию silent mode. |
| pass | Это был такой workaround на время. |
| fail | У нас будет стендап в десять утра. |
| pass | Поставь режим manual, а не automatic. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра, а потом закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |

#### google_gemma-3-12b-it-Q4_K_M — B_rule

| | Output |
|---|---|
| pass | Ну, лучше тогда automatic, потому что у меня люди в помещении этой же квартиры, тут может ребенок закричать. В этом плане, наверное, automatic лучше, чем dynamic. |
| pass | У этого микрофона есть режим dynamic и режим cardioid. |
| pass | Дай мне feedback по этому pull request. |
| pass | Включи в настройках опцию silent mode. |
| pass | Это был такой workaround на время. |
| fail | У нас будет стендап в десять утра. |
| pass | Поставь режим manual, а не automatic. |
| pass | Менеджер попросил перезагрузить компьютер и проверить интернет. |
| pass | Мы деплоим релиз завтра, а потом закомитим фикс. |
| pass | Скинь мне файл в чат и поставь лайк. |
