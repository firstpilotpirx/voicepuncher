# Design

No architectural decision. The sign's placement is the existing pure placement function of the macOS sign window; it takes a right offset and a down offset of its own (8 pt and 6 pt) instead of one shared 16 pt gap.

## Test levels

| Scenario | Level |
|---|---|
| A hotkey recording shows the dictation sign | component `.feature` (unchanged) |
| A recording started by voice shows the dictation sign | component `.feature` (unchanged) |
| The dictation sign sits close to the pointer's tip | unit (placement math) + manual run |
| Processing shows no dictation sign | component `.feature` (unchanged) |
