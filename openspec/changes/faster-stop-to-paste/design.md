# Design

## Context

Live chunking already transcribes chunks while the user speaks, cuts them in a pause and merges the overlap at the seams; only the 30 s default threshold keeps a typical dictation in one chunk. The settings window already has a Language pop-up (17 languages and Detect automatically) that adds a value the file sets outside its list as an extra entry.

## Goals / Non-Goals

**Goals:** shorter wait after "stop" through two defaults and a narrower Language choice.

**Non-Goals:** a new cut or merge algorithm; changing the overlap (2 s) or the pause lookback (5 s); rewriting existing configuration files.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Chunk threshold | 10 s | the user picked it from the measured table: 0.63 s for the last piece against 1.16 s at 30 s; below 7 s the time stops falling while seams multiply | 7 s, 15 s |
| Default language | `ru` | the user always speaks Russian; English inclusions and English-only speech came out right with `ru` | `auto` (+0.5 s per request) |
| Language choice | Russian, English, Detect automatically; another code from the file shown as its own entry, as the pop-up already does | the user speaks only these two; an existing file is never changed unasked | refusing other codes |
| Accuracy and seam stages | keep running on the defaults | they then measure what the user gets: 10 s chunks and Russian; the 100-recording sample runs because the defaults sit on the recognition path | pinning the old defaults in the benchmark |
| Pause lookback with 10 s chunks | unchanged, 5 s | the cut still falls in the second half of the chunk, after the 2 s overlap | scaling it with the threshold |

## Risks / Trade-offs

- Three times more seams per dictation: a merge error costs a word more often. The seam sweep and matrix (`e2e-seams`) run in `full-check`, since the threshold is on the chunking path.
- English-only corpus recordings now go out as Russian; if the accuracy ratchet fails on them, the benchmark asks for each recording's own language (the corpus knows it), as a user who dictates English picks English in settings.
- E2E steps that assumed `auto` by default set it in their own configuration.

## Test levels

| Scenario | Level |
|---|---|
| Automatic language detection; Fixed language; Short Russian speech is never transcribed as Korean; English speech stays English with automatic detection | unchanged (E2E) |
| Russian is the language when none is configured | component `.feature` (`speech_recognition.feature`, fake service records the request) |
| English speech stays English with the default language | E2E `.feature` |
| A long dictation is transcribed while it goes on | component `.feature` (`speech_recognition.feature`) |
| A short dictation is one chunk | component `.feature` (`speech_recognition.feature`) |
| The language choice offers three entries | component `.feature` (`settings_window.feature`) |
| Russian is shown when the file names no language | component `.feature` (`settings_window.feature`) |
| Another language from the file is kept | component `.feature` (`settings_window.feature`) |
