## Context

The decision of `command-spotting/src/streaming/decide/` reports a one-word command only when it is the whole utterance or ends it, and then only once settled: the gate's pause began within 0.35 s after the word (`SPEECH_AFTER_SECS`) and the recognizer decoded 0.2 s past it (`SETTLE_SECS`), or the app's sync stands for the pause, or the utterance closed. In long speech the gate hears that pause late, so the report trails the word by 1–1.4 s (service log of 2026-11-09). Doubled forms and chains already count anywhere at once (`placed`). The app needs no change: it ends a dictation at the command's start (`truncate_at`), starts one at its end, and strips the command words of any non-alone placement from the text.

## Decisions

### D1: a command word counts anywhere once decoded past it

A one-word form (or a phrase ending in one) found anywhere among the fresh words, in the state's accepted set, is reported as soon as the recognizer decoded `SETTLE_SECS` (0.2 s) past its end or a later word: no gate pause, no sync, no close. The 0.2 s stays, as a streaming draft may still grow ("stop" → "stopped"); a word that may be the end of a longer command word ("start" of "restart") keeps waiting for the close as today. The finding's placement is `Tail` when the command ends the fresh words, `Anywhere` otherwise, so the app strips its words from a dictation as now. `alone` and `ending` fold into this path; `settled` loses its pause clause. Rejected: reporting at the first draft of the word, as the drafts of both recognizers change within 0.1–0.2 s and the matcher wants the exact word.

### D2: an English-only command where Russian heard another word keeps the old rule

The English recognizer hears Russian speech as English words ("чтобы" → "stop", user's recording). While a dictation runs ("stop dictation" is listened for), when only the at-once rule of D1 finds an English one-word command and the Russian recognizer holds no command word over the same span (another word, or nothing decoded past it yet while the utterance is open), the finding waits for the gate's pause as before and is no command at the head of a sentence that goes on. The rules that counted a command before (chains, doubled forms, a start opening speech, a command at the pause) go first and are never doubted; outside a dictation the user speaks commands, not Russian sentences, so nothing is doubted there ("Enter the building on the left" presses Return). The Russian recognizer's last words are already kept per utterance (`last`), so this needs no new state. A command the Russian recognizer hears (respelled "стоп", "энтер") goes out at once by D1: it decides most reports today (246 of 294 in the log).

### D3: a repeat within 0.6 s is the same command

With the first word reported at once, the second word of a doubled form arrives after its report. `doubles` already drops the same commands within `DOUBLE_GAP_SECS` (0.6 s), so "enter enter" presses Return once; the doubled form's own match (`placed`) still runs first when both words are decoded together.

### D4: measured on the user's recordings

`just streaming-bench --recorded` before and after, real time: every recorded command still recognized once, and the median delay of commands said right after speech at most 0.4 s.

| Run (6 sessions, real time) | right | false alarms | delay median | p90 | start median | stop median |
|---|---|---|---|---|---|---|
| before (`main` at 3f2c0a2) | 51/53 | 4/9 | 485 ms | 5154 ms | 445 ms | 436 ms |

## Risks

- Command words inside dictated speech now trigger: accepted by the user, who keeps them out of dictation.
- A chain said after "stop" relies on the app's new listening set arriving before the next word decodes; `chain_tail` keeps matching the chain over every word, so its tail is not lost.

## Test levels

| Scenario | Level |
|---|---|
| A command phrase spoken on its own is recognized | E2E `.feature` (existing) |
| A command said after a long time of listening is recognized | E2E `.feature` (existing) |
| A command word inside a sentence counts while listening | unit (decide) + E2E `.feature` |
| Ordinary speech triggers no command | E2E `.feature` (existing) |
| A stop word inside a dictated sentence ends the dictation at once | session test + E2E `.feature` |
| Stop right after the last dictated word | E2E `.feature` (existing) |
| Enter right after other speech | E2E `.feature` (existing) |
| Stop after a long dictation counts without waiting for a pause | session test (scripted recognizers, gate still hearing speech) |
| Start followed by the dictation at once | E2E `.feature` (existing) |
| Enter followed by speech presses Return | unit (decide) + E2E `.feature` |
| A short form is recognized as its command | E2E `.feature` (existing) |
| Cancel alone discards a dictation | E2E `.feature` (existing) |
| A long form counts by its command word | E2E `.feature` |
| A long form ending in a command word counts by that word | E2E `.feature` (existing) |
| Saying paste text pastes the clipboard at the cursor | component + E2E `.feature` (existing) |
| Paste leaves the clipboard as it was | component `.feature` (existing) |
| A paste word inside a sentence pastes at once | unit (decide) + E2E `.feature` |
| Saying paste during a dictation pastes the clipboard and the dictation goes on | component `.feature` (existing) |
| A dictation with a paste in it pastes its text without the command | component `.feature` (existing) |
| Start said alone is reported within its bound | E2E `.feature` `@cues` (existing) |
| Stop said alone is reported at once | E2E `.feature` `@cues` |
| Stop opening a sentence counts at once | unit (decide) + E2E `.feature` |
| A Russian sentence heard as opening with stop is no command | unit (decide, D2) + E2E `@voice_samples` (existing) |
| Press enter after a sentence is reported without waiting for the pause | E2E `.feature` `@cues` |
| The user's recorded commands are reported within their bound | E2E `@voice_samples` (existing) |
| A command after speech in the user's recordings is reported soon after its word | bench (`just streaming-bench --recorded`), recorded in § D4 |
| Start start then silence starts at once | E2E `.feature` `@cues` (existing) |
| Stop stop in running speech stops at once | E2E `.feature` (existing) |
| Start start inside a sentence starts at once | E2E `.feature` (existing) |
| Enter enter presses Return once | unit (decide, doubles) + E2E `.feature` |
| A chain the states do not allow carries out what the state accepts | E2E `.feature` |
| Stop enter pastes and sends | E2E `.feature` (existing) |
| Stop enter start sends and starts the next dictation | E2E `.feature` (existing) |
| Cancel clear discards and clears | E2E `.feature` (existing) |
| The end of a chain heard only when the utterance ends still counts | unit (decide, existing) |
| The last command of a chain decoded after the first ones were carried out still counts | session test (existing) |
| The other recognizer's words of a chain carried out are not carried out again | session test (existing) |
| The user's recorded chains keep their last command | E2E `@voice_samples` (existing) |
