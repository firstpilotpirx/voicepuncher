# Streaming bench results

`just streaming-bench` (real time, 20 ms chunks, 1 s of silence after each clip) over the labelled
clips of `~/.voicepuncher/research/command-bench/dataset.jsonl`: 415 clips with commands (voice
samples and confident history entries), 101 without. Delay: from the end of the command's words
to its report, wall clock, clips with one command.

| Run | Setup | Right | False alarms | Delay median | p90 |
|---|---|---|---|---|---|
| 1 | baseline: v0.42.0, Nemotron 160 ms int8, 2 threads, per-frame join | 380/415 (91.6 %) | 6/101 | 575 ms | 1364 ms |
| 2 | one decode thread per recognizer | 377/415 (90.8 %) | 6/101 | 376 ms | 1368 ms |

Misses of run 1: "start" 9, "enter" 8, "stop" 5, "clear" 3, "paste" 2, others 1 each.

Run 2: the median fell by a third; the tail stays, which is the lone command waiting to settle and the utterance's close.

## English recognizer: threads and models

Every fourth clip (`--sample 4`: 104 with commands, 25 without), real time, one decode thread per
recognizer.

| English model | Threads | Right | Delay median | p90 |
|---|---|---|---|---|
| Nemotron 0.6B 160 ms | 2 | 87.5 % | 252 ms | 884 ms |
| Nemotron 0.6B 160 ms | 4 | 87.5 % | 135 ms | 853 ms |
| Nemotron 0.6B 160 ms | 6 | 87.5 % | 149 ms | 865 ms |
| Nemotron 0.6B 80 ms | 4 | 87.5 % | 291 ms | 2065 ms |
| Nemotron 3.5 160 ms | 4 | 78.8 % | 94 ms | 769 ms |
| Nemotron 3.5 80 ms | 4 | 78.8 % | 230 ms | 1017 ms |

Chosen: Nemotron 0.6B 160 ms at 4 threads. 6 threads add nothing; the 80 ms chunk decodes more
often and falls behind; Nemotron 3.5 is faster but misses one command in ten more.

## The lone "stop"

The user's lone "stop" came ~0.8–1.2 s after the words. The English model heard nothing of it;
the Russian model held "сто" until ~0.65 s of audio past the word before it emitted "п". At the
gate's pause the session now feeds 0.6 s of silence into every recognizer ahead of the stream
(the flush) and maps the recognizers' times back to the stream's.

The bench streams 20 ms chunks, shorter than the gate's 32 ms window: a chunk that completed no
window counted as silence, opening a pause (and a flush) inside words. Such a chunk now leaves
the gate's state as it was.

Flush A/B on the fixed bench (Nemotron 160 ms, 4 threads):

| Setup | Every fourth clip: right | Median | p90 | "stop": right | Median | p90 |
|---|---|---|---|---|---|---|
| no flush | 91/104 (87.5 %) | 208 ms | 881 ms | 30/36 | 412 ms | 1158 ms |
| flush 0.6 s | 90/104 (86.5 %) | 154 ms | 634 ms | 30/36 | 208 ms | 1170 ms |

The flush costs one long dictation whose final "enter" the English model then heard as "enter
render"; it halves the lone "stop" and cuts the tail by a quarter. Kept.

Not benched: decode threads at user-interactive QoS and Parakeet Realtime EOU 120M; 4 threads
already put the English decode well under real time, so the delay left is the models' own
lookahead, which the flush addresses.

## Utterance tail

With the flush, the tail past the pause (`TAIL_SECS`) on every fourth clip:

| Tail | Right | Median | p90 |
|---|---|---|---|
| 0.5 s | 90/104 | 154 ms | 634 ms |
| 0.3 s | 90/104 | 158 ms | 721 ms |
| 0.1 s | 91/104 | 147 ms | 780 ms |

No command is lost at 0.1 s (one more is heard); the p90 moves within the run-to-run spread.
Chosen: 0.1 s, so an utterance closes 0.4 s after its speech instead of 0.8 s.

## Mishearings

The misses of the whole bench, as the Russian recognizer wrote the command:

| Written | Command | Where | Taken |
|---|---|---|---|
| «тартарт», «стартарт», «тарстард» | start, doubled | history | yes: "с" may drop, "т" may sound "д", and a glued double is split with backtracking ("тар" + "тарт") |
| «лентер», «яндер» | enter | history | yes |
| «поист» | paste | history | yes |
| «клио», «клет» | clear | voice samples | yes: "р" may drop |
| «центр», «клевер», «сто» | enter, clear, stop | history, voice samples | no: Russian words |
| «интернтер», «терендер» | enter | history | no: no single reading |

The English recognizer's misses ("Store objectation", "To dictation", "Capture dictation",
"Lectation", "Addictation") are left: the matcher's near forms already take "capsule
dictation" for cancel, and widening them reads one command as another.

Scored against the commands each voice sample states (`forms.jsonl`, not the words said): 105
of 115 clips before relabelling — 103 before the new spellings.

## A command ending a sentence, and the user's recorded sessions

A command ending a sentence ("we meet on Friday at ten stop") was reported only when the
utterance closed. It is now reported at the gate's pause once the recognizer decoded up to it,
like a command said alone. «сто» right before "please" is taken for "stop" («сто плис»); alone
it stays a Russian word.

`just streaming-bench --recorded ~/.voicepuncher/voice-samples` cuts every labelled row of the
user's five recorded sessions (0.6 s before, 1 s after, within its neighbours) and listens for
the commands of the state it was said in, as the app does.

| Run | Recorded: right | Start/stop within 0.3 s | within 0.6 s | "stop" median | p90 | Bench: right | False alarms |
|---|---|---|---|---|---|---|---|
| before | 102/114 | 13/21 (two sessions) | 16/21 | 373 ms | 996 ms | 382/415 | 7/101 |
| sentence end at the pause, «сто плис» | 102/114 (89.5 %) | 28/38 | 35/38 | 151 ms | 387 ms | 393/415 (94.7 %) | 8/101 |

The misses left are the models' own: "Dictation" or "The obdictation" for the whole command,
"start" heard for "restart", and two chains ("stop clear", "cancel clear") whose second command
belongs to standby while the app listens for the dictation's commands. The accuracy bar is the
measured share less two clips of run-to-run spread: 87 of 100.

## Gate gain and threshold

Fast runs (`--fast`) of the whole bench and of the recorded sessions, the gate's gain and
speech threshold swept:

| Gain | Threshold | Bench: right | False alarms | Recorded: right |
|---|---|---|---|---|
| 1.5 | 0.5 (before) | 393/415 | 8/101 | 98/114 |
| 1.5 | 0.4 | 392/415 | 9/101 | 98/114 |
| 1.5 | 0.3 | 395/415 | 6/101 | 99/114 |
| 3.0 | 0.5 | 396/415 | 8/101 | 99/114 |
| 3.0 | 0.4 | 396/415 | 7/101 | 99/114 |
| 3.0 | 0.3 | 400/415 | 6/101 | 100/114 |
| 3.0 | 0.2 | 394/415 | 5/101 | 100/114 |
| 4.5 | 0.3 | 400/415 | 6/101 | 100/114 |
| 4.5 | 0.4 | 399/415 | 7/101 | 100/114 |

Chosen: gain 3.0, threshold 0.3 — the most commands heard with fewer false alarms than before;
a louder gain adds nothing.

Real time with gain 3.0, threshold 0.3: recorded 104/114 (91.2 %), "stop" median 147 ms, p90
363 ms; bench 396/415 (95.4 %), false alarms 7/101, median 176 ms. The accuracy bar becomes the
new share less two clips: 89 of 100.

At a pause the English recognizer may still hold a draft ("Let's start" for the user's quiet
"restart"), so a command ending a sentence now counts once the recognizer decoded 0.2 s past
it, which the flush's silence reaches within milliseconds. Real time, recorded / bench:

| Replay before the onset | Recorded: right | "start" median | Bench: right | median | "start" median |
|---|---|---|---|---|---|
| 0.5 s | 104/114 | −422 ms | 396/415 | 177 ms | 39 ms |
| 0.8 s | 105/114 | −120 ms | 396/415 | 289 ms | 274 ms |

The longer replay hears one more command but delays every start by a quarter second: the
replay stays 0.5 s.
