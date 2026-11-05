## Context

The service reports a chain as soon as one recognizer decoded it: `decide` in `command-spotting/src/streaming/decide/mod.rs` judges only the *fresh* words of a recognizer's text, those starting at or after the end of the utterance's last report (`Utterance::reported_until`, shared by both recognizers), and the chain matcher (`PhraseMatcher::anywhere`) needs the chain's words together. The English recognizer emits a word only once more audio came after it, so the last command of a chain shows up a decode or more after the ones before it. Traced on the user's recording session-4/5 ("that is all stop enter start", bench in real time, `RUST_LOG=…session=info` with a temporary print of each decode's words): the Russian recognizer reported "stop" (its stamp 1.72–2.10 s); the English one then had "stop 2.24–2.34, enter 2.88–3.06" fresh and reported "stop dictation, press enter" (its "stop" doubled: the peer `dedupe-close-reports-across-recognizers`); the Russian one decoded "start 2.76–3.18" and the English one "start 3.20–3.24" after that, but with the report's end at 3.06 the Russian "start" was not fresh and the English one stood alone among the fresh words, where "start" while dictating is no command (it is accepted only after "stop" or "enter", `accepted_after`). Every later decode up to the close saw the same lone word: the tail was lost in 3 real-time runs of 3. Sessions 4/6 and 4/8 keep their tail because the Russian recognizer hears "энтерстарт" as one word and reports the chain whole; 4/7 keeps it because "enter" alone is a command while dictating.

## Goals / Non-Goals

- Goals: the last command of a chain decoded after the ones before it were reported is reported once, after them, from the first recognizer whose words say so; no report changes for chains decoded whole; measured before and after on the user's recorded chains.
- Non-goals: a command the two recognizers report doubled with different stamps (`dedupe-close-reports-across-recognizers`), the bench's `--fast` mode (`fast-bench-reproduces-app-timing`), the English recognizer's lag, a new E2E timing bound for chains.

## Decisions

**D1. A chain is matched over every word of the utterance, and its commands said after the last report are the tail.** `decide` gets one more step, after `placed` (over the fresh words, unchanged) and before `alone` and `ending`: `chain_tail` runs `PhraseMatcher::anywhere` over *all* the recognizer's words and keeps the commands of the chain whose first word starts at or after `reported_until`; it reports them (`Placement::Anywhere`, at once, as a chain decoded whole is reported today) when at least one command was dropped and at least one is left; with nothing dropped `placed` already judged the same words, and a chain reported whole leaves nothing. The reported commands are those whose first word starts before the bound, the same freshness rule `decide` applies to words, so a word stamped across the bound (the English recognizer stamps a word 0.65–1.1 s later than the Russian one, peer note of 2026-11-05) never comes out doubled from this path: on session-4/5 the Russian "start" (2.76 s, before the bound 3.06 s) is dropped, the English "start" (3.20 s) is reported at its first decode, 3.24 s, 20 ms after the speech end; the Russian one would have been earlier, but a rule by the span's end reports "stop dictation, press enter" again whenever the English "stop" straddles the Russian report's end. The alternative, teaching the matcher the last reported command so a lone fresh word accepted after it counts, needs the report inside the matcher's state; kept out. `before_a_pause` calls `decide` with the bound 0.0, so `chain_tail` drops nothing there and stays silent. `doubles` catches the other recognizer's hearing of the same tail (same commands within `DOUBLE_GAP_SECS` of the report), as for any command.

**D2. The session test scripts the late tail.** In `command-spotting/src/streaming/session/tests/`, the English `Scripted` recognizer says "stop" 0.6 s, "enter" 0.9 s and "start" 1.2 s into the utterance with its usual lag, the Russian one hears "стоп" and "энтер" only, the audio is fed in 20 ms frames so the decodes see "stop enter" before "start" (as `should_report_a_command_one_recognizer_heard_while_the_other_still_decodes` feeds it); expected: the reports `["stop dictation", "press enter"]` then `["start dictation"]`, nothing else. A `decide` unit test holds the session-4/5 words and stamps from the trace with the bound at 3.06 s and expects "start dictation" starting at 3.20 s; a second one with the bound at the chain's end expects nothing. Both are red before D1.

**D3. The user's recorded chains in E2E.** A `@smoke @voice_samples` scenario in `e2e-test/tests/features/voice_control.feature` under the rule "A chain of commands is carried out in order" plays each recorded chain ending in "start" (session-4 rows "that is all stop enter start" twice and "never mind cancel enter start", by their labels in `forms.jsonl`) cut with the margins the recorded steps use, in real time through the app as `carried_out` does, while dictating, and expects every command of its label in order, each once (3 clips of about 4 s: about 15 s of real time). The chains without "start" ("stop clear", "stop enter", "that is all stop enter", "cancel clear") end in a command accepted alone and keep their tail today (session-4/7); they stay covered by the `voice_samples` tests and `just streaming-bench --recorded`.

**D4. Measured before and after.** Before (this branch, v0.46.2 code): session 4 in real time, 3 runs: 4/5 lost "start" 3 of 3 (run 3 also reported "stop" twice); 4/6, 4/7, 4/8 kept their tail; every recorded session in real time: see the table below. After D1: the same runs, session 4 three times and every session once; the acceptance bar holds when no chain loses its tail and no other clip gets worse.

| run | before | after |
|---|---|---|
| session 4 ×3 real time, chain tails lost | 3 (all 4/5) | – |
| sessions 1–6 real time, wrong clips | – | – |

## Test levels

| Spec scenario | Test |
|---|---|
| The last command of a chain decoded after the first ones were carried out still counts | `decide` unit tests (D2, red first); session test with scripted recognizers (D2, red first) |
| The user's recorded chains keep their last command | E2E `@smoke @voice_samples` scenario (D3) |
| The end of a chain heard only when the utterance ends still counts | the session test of D2 covers it at the close too: the Russian recognizer never hears "start", the English one emits it at its final decode at the latest |
| Stop enter start sends and starts the next dictation; Stop enter pastes and sends; Cancel clear discards and clears; A chain the states do not allow is no command | existing E2E chain scenarios (`@smoke` and `@regression` rows), unchanged, rerun in `check` and `full-check` |

## Risks

- `dedupe-close-reports-across-recognizers` edits `Utterance::report` and the session tests at the same time, and targets 0.46.3 too: whichever lands second merges `main` at `report` (D1 touches `decide`, not `report`) and takes the next version (`kit version-check`).
- A tail word the English recognizer stamps before the chain's reported end (its stamps are late, so unlikely) is dropped by D1 and the tail is lost as today; the bench runs of D4 show whether any recorded clip does this.
