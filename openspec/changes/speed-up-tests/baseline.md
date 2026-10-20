# Baseline and results

## Stage medians before (stage journal, 14–20 Oct, passed runs only)

| Recipe | Stage | Runs | Median |
|---|---|---|---|
| test | test | 88 | 101.1 s |
| check | test | 34 | 70.0 s |
| check | e2e-smoke | 19 | 365.0 s |
| check | e2e-quick | 19 | 41.6 s |
| check | refine-eval | 19 | 61.1 s |
| full-check | e2e-smoke | 25 | 366.7 s |
| full-check | e2e-seams | 17 | 352.8 s |
| full-check | e2e-resilience | 17 | 195.5 s |
| full-check | e2e-bench | 16 | 209.8 s |
| full-check | refine-eval | 17 | 58.8 s |

## Slowest tests of `just test` before

| Test | Time |
|---|---|
| `commandspotting_reconnect_it` long outage | 61–65 s (nextest.toml note) |
| `cucumber_integration` scenarios | 59.4 s |
| `transcription_openai_it` give-up within half a minute | 22.9 s |
| `placed_commands_it` same chain at any confirmation speed | 21.6 s |
| `placed_commands_it` restart once while confirmations are slow | 17.3 s |
| `server_it` keep answering while a confirmation hangs | 15.8 s |

## Removed tests and the tests that keep their behavior

(filled in by task group 6)
