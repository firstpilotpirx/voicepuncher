# Design

No architectural decision.

## Decisions

### 1. "Capsule dictation" is an alias of "cancel dictation"

The command spotting service's `PhraseMatcher` already maps known slips of the speech recognition service for commands to their command (`ALIASES`: "star dictation", "stuck dictation", "presenter", "clear text"). "Capsule dictation" joins that list. It is two words away from no other command phrase, so it cannot be taken for another command.

### 2. A "please" opening a voice-started dictation is dropped with the command words

`command_words::without_leading_command` already drops the start or restart phrase that opens the dictation, with a "please" right after it. When the head check confirmed the command before the "please" was spoken, the recording starts after "dictation" and the text opens with "please". The function also drops a "please" ("пожалуйста") that opens the text after a voice start, when more text follows, and capitalizes what follows. A dictation started by the hotkey is untouched.

## Test levels

| Scenario | Level |
|---|---|
| Cancel dictation heard as capsule dictation discards the dictation | unit (`command-spotting/src/matcher.rs`); E2E "A hands-free session in a synthesized voice" (10 runs in a row) |
| Please after start dictation is not pasted | component (`app/tests/features/component/voice_control.feature`); unit (`app/src/core/domain/command_words.rs`) |
| Press enter please presses Return | unchanged: E2E (`e2e-test/tests/features/voice_control.feature`), integration (`command-spotting/tests/engine_it.rs`) |
| Stop dictation please pastes the dictated text | unchanged: E2E, integration (`engine_it.rs`) |
| Every recorded command of the user is recognized | unchanged: E2E `@voice_samples`, integration (`engine_it.rs`) |
| A short Russian phrase is no command | unchanged: E2E |
| The user's quiet restart after any pause before it | unchanged: E2E |
| The user's quiet restart, a short pause, then speech | unchanged: E2E |
| A command said in a quiet voice is recognized | unchanged: E2E, integration (`engine_it.rs`) |
| A quiet clear all text clears the text | unchanged: E2E |
