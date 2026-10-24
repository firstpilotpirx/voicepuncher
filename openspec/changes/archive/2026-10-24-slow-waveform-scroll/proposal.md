## Why

The recording pill's level waveform crawls from right to left too fast for the user: a new column enters every 100 ms, so the strip moves 40 pt a second. The user likes the pill's width (727 pt, as long as ChatGPT's dictation bar, shipped in 0.38.5) and asks only that the waveform move about 10 % slower.

## What Changes

- A new column enters every 110 ms instead of 100 ms, and the strip slides one pitch (4 pt) per column, so it moves about 36 pt a second instead of 40.
- The column count (160, 148 beside the commands-unavailable sign) and the pitch stay, so the pill keeps its 727 pt width; the waveform now shows the last 17.6 s of the recording instead of 16 s.

Current stage: PoC. Target stage: PoC.
Scope: the waveform's column period and the history length that follows from it.
Acceptance bar: one second of speech adds nine columns instead of ten; speech stays visible for about 17.6 s; the pill is still 727 pt wide; the unit tests of the level history and the component tests of the overlay pass.
Out of scope: the pill's width and height, the column count, pitch and heights, the colours, the smoothing of the level.

## Capabilities

### Modified Capabilities

- `visual-feedback`: the recording waveform adds a column every 110 ms instead of 100 ms and keeps the last 17.6 s instead of 16 s.

## Impact

- `app/src/core/domain/level_history.rs` (`COLUMN_PERIOD`, the comment of `FULL_CAPACITY`, its unit tests)

## Open points

- The first request said about 20 %; the user then asked for 10 %. Proposed choice: exactly 10 % slower, 110 ms a column.

## Release

SemVer: PATCH — a visual adjustment of an existing capability.
Target version: 0.38.6.
