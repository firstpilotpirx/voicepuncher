## Decisions

### D1. "paste text", short form "paste", English only
Same shape as every command: full phrase, one-word short form, optional "please". The keyword model is English-only, so no «вставь».

### D2. Cmd+V through `KeyPresser`, never through the clipboard
`KeyPresser` gains `paste()`, posting Cmd+V like the other key presses. The command never reads or writes the clipboard: the focused app pastes whatever it holds (text, image, files), an empty clipboard pastes nothing, dictation keeps its own clipboard restore. Rejected: the `Paster` port — it writes its text to the clipboard first.

### D3. A standby command, like "clear all text"
"paste text" joins the standby phrases in the app and in the service (`placed.rs`), so it gets the placements and chains of "enter" and "clear" with no new rule ("paste paste", "paste enter", "stop paste enter"). During a dictation it is ignored: "paste" there is likely a dictated word.

### D4. Own sound and sign
Default sound `Frog` (no other cue uses it), `paste_sound` in `[sounds]`; overlay sign `⎘ PASTE`.

### D5. Keywords like "clear all text"
`keywords.txt` gets "PASTE TEXT" (with and without "PLEASE") at boost 1.5, threshold 0.20. The short form "paste" maps to its phrase in the confirmation's transcript, like "enter" and "clear".

## Risks

- A misheard "paste" ("based", "taste") is a missed command, never a wrong one: one-word forms must match exactly.
- No recording of the user's own "paste" yet; `voice_samples_it` does not cover it.

## Test levels

| Scenario | Level |
|---|---|
| Saying paste text pastes the clipboard at the cursor | component `.feature` + E2E `.feature` (recognized as "paste text") |
| Paste leaves the clipboard as it was | component `.feature` (no clipboard write, only Cmd+V) |
| A paste phrase inside a sentence is ignored | E2E `.feature` |
| Paste during a dictation is ignored | component `.feature` |
| A short form is recognized as its command ("paste") | E2E `.feature` |
| Paste enter pastes the clipboard and sends | unit (`placed.rs`) + component `.feature` + E2E `.feature` |
| Paste sounds different from enter and clear by default | unit (`config.rs` defaults) + component `.feature` |
| Stop dictation without a dictation is ignored | component `.feature` + E2E `.feature` (existing) |
| Clear during a dictation is ignored | component `.feature` (existing) |
| Start or let's start on its own during a dictation is ignored | E2E `.feature` (existing) |
| Start ending a dictated sentence is ignored | E2E `.feature` (existing) |
| Start inside a dictated sentence is ignored | E2E `.feature` (existing) |
| Cancel alone discards a dictation | E2E `.feature` short-form table (existing) |
| Stop stop in the middle of speech | component `.feature` (existing) |
| Enter enter in the middle of speech | E2E `.feature` doubled table (existing), "paste paste" row added |
| Cancel cancel in the middle of speech | component `.feature` (existing) |
| Stop clear enter | component `.feature` (existing) |
| Enter and clear sound different by default | component `.feature` (existing) |
| A sound the user picked for a command plays | unit (`config.rs`), `paste_sound` case added |
| Enter start shows the recording right after the enter sign | component `.feature` (existing) |
| Stop clear enter ends on the enter sign | component `.feature` (existing) |
| Paste sign names the command | unit (overlay sign table) |
| Cmd+V is the paste key press, untrusted fails | unit (macOS key presser) |
