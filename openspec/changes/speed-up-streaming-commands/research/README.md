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
