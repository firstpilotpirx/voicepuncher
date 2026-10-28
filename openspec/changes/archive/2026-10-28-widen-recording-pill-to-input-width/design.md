# Design

No architectural decision.

The pill width is the widest of two recording rows plus its padding: the full waveform beside the record ring, and the shorter waveform beside the record ring, a separator and the commands-unavailable sign. A waveform of n columns at the 4 pt pitch is (n − 1) × 4 + 2 pt wide. With 210 and 196 columns the rows come to 922 pt and 919 pt, so the pill is exactly 922 pt and 210 columns of 110 ms show the last 23.1 s. The proposal's 209 and 197 would make the row beside the sign the wider one, at 923 pt.

The scenario names "The waveform keeps the last six seconds" and "The recording pill is as long as ChatGPT's dictation bar" stay as they are: a MODIFIED requirement keeps every scenario name of the current spec; their steps carry the new numbers.

## Test levels

| Scenario | Level |
|---|---|
| Speech leaves tall columns that slide to the left | unit (`level_history`), unchanged |
| Silence leaves a dotted line | unit, unchanged |
| Quiet and loud speech leave columns of different heights | unit, unchanged |
| A new recording starts with a dotted waveform | unit, unchanged |
| The waveform shows together with the record sign | component `.feature`, unchanged |
| The waveform still shows speech from 22.5 seconds ago | unit (`level_history`): one second of speech, then 22.5 s of silence |
| The waveform keeps the last six seconds | unit (`level_history`): 20 s of speech, then 24 s of silence |
| The waveform moves at half its former pace | unit, unchanged |
| The record dot pulses in silence too | unit, unchanged |
| The recording pill is as long as ChatGPT's dictation bar | component `.feature` (922 points) and unit (`pill_content`, `waveform` widths); manual run: the user compares the pill with the input box |
