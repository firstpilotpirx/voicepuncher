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
