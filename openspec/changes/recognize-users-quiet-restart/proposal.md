# Proposal

## Why

The user's quiet "restart" does not start a dictation over: the speech recognition service hears it as "Let's start." This breaks two promises of the spec: "restart" said on its own starts the dictation over, and every command in the user's recordings is recognized exactly once, apart from recordings marked unclear (this one is not).

- **What is heard.** In the clip the command spotting service sends to confirm the user's recorded "restart" in `~/.voicepuncher/voice-samples/session-2.wav` (27.23–27.52 s), the speech recognition service hears "Let's start." whenever the clip holds the 0.5 s of faint sound the user made right before a weak "re" (from 26.70 s), and "Start." when it holds none of it. The user's stream always holds all of it, so during a dictation this restart fails every time; `voice_forms_it` lists it as known wrong since `main` went red on it.
- **Not the level alone, but what comes before the word.** The user speaks quietly, but this "restart" is no quieter than the two other recorded "restart"s in that session, which are heard as "Restart." every time. What sets it apart is the faint sound before the "re".
- **Neither "Let's start." nor "Start." is a command during a dictation.** "let's start" is no command phrase, and "start" is accepted only when no dictation runs. So the restart takes no effect: the dictation goes on with the text the user meant to throw away.
- **The prompt does not fix it.** `restore-recognition-prompt` measured confirmations with the command phrases as a prompt: this restart was then heard right, but the user's other speech gave three false commands, Russian counting words became commands and the slowest confirmations took 1.4 s instead of 0.16 s. Confirmations stay without a prompt.
- **The tests leave it out.** `fix-short-pause-after-command` excludes this recording from its E2E scenarios (`UNSTEADY_RECORDINGS` in `e2e-test/tests/steps/voice_control/session/speech_after_start.rs`), and `voice_forms_it` holds it in `KNOWN_WRONG`; both are left to this change.

## What Changes

- **Measured first (done, design.md § Results).** On `main` with `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt`, the recording is heard as "Let's start." or "Start." depending only on how much sound before the "re" the clip holds. Cut 0.2–0.3 s before the "re" and raised to a normal level, it is heard as "Restart." after any silence from 0 to 0.8 s.
- **A steady clip for every short utterance was measured and rejected.** Sending that clip — a fixed lead before the utterance's first sound, raised toward a normal level — in place of today's for every short utterance fixes this restart, but changes what is heard for about 30 of the user's 70 short recorded forms, and with every lead tried breaks some of them ("restart dictation" heard as "That is tar dictation.", "restart restart" as "restart and start", "start please" as "Start the please.").
- **The fix: a second hearing of a near miss of "restart".** When the confirmation of a short utterance during a dictation hears no command, but a sentence of at most three words ending in "start" ("Start.", "Let's start."), and "restart dictation" is accepted in that state, the utterance is heard once more through the steady clip: cut a fixed lead before its first sound and raised toward a normal level when quiet. It counts as "restart dictation" only when that second hearing, as a whole, says it. Nothing else changes: every other confirmation sends the clip it sends today, so no other recorded form can be heard differently. The cost is one more request, about 0.15 s, for a near miss only.
- **Rejected: taking "let's start" or "start" as "restart" during a dictation.** Both are ordinary words in a dictation, and a restart throws away all that was dictated: a wrong restart loses the user's text, while a missed one only has to be said again. "start" stays no command during a dictation, and the second hearing must say "restart" itself: "start" and "let's start" said on their own, at a normal level and as quietly as the user speaks, are heard as themselves in it.
- **Tests:** the `KNOWN_WRONG` entry and the `UNSTEADY_RECORDINGS` exclusion are removed, so `voice_forms_it` and the scenario "The user's recorded command alone, a short pause, then speech" of `fix-short-pause-after-command` run on this recording; a test plays it during a dictation after each silence from 0.3 to 0.8 s; every command phrase and short form said on its own in a quiet voice takes effect; "let's start" and "start" said during a dictation (on their own, ending a sentence and inside one, at a normal level and as quietly as the user speaks) neither restart nor end it.
- No change to which phrases count as commands or to the states that accept them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: the user's own way of saying the commands is recognized whatever the pause before a command, in a quiet voice too; "let's start" and "start" said during a dictation neither restart nor end it.

## Scope

- Current stage: PoC. Target stage: PoC, with the user's quiet "restart" taking effect after any pause.
- In scope: the second hearing of a near miss of "restart" during a dictation and its steady clip; removing the exclusion and the known-wrong entry; the tests above.
- Lands after `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt`, all three on `main`.
- Acceptance bar:
  - The user's recorded "restart" at 27.23 s in `session-2` starts a running dictation over after every silence from 0.3 to 0.8 s, and when 0.3–0.6 s of silence and speech follow it (the scenario of `fix-short-pause-after-command`, without the exclusion).
  - Every recorded command of the user is still recognized exactly once (`voice_samples_it`, `voice_forms_it` with no known-wrong entry), and the E2E voice control scenarios pass.
  - Each command phrase and short form, said on its own as quietly as the user's quietest recorded commands, takes effect in the state that accepts it.
  - "let's start" and "start", said during a dictation on their own, ending a sentence and inside one, at a normal level and as quietly as the user speaks, neither restart nor end it.
  - No command takes effect in the existing negative cases: short Russian phrases, a command word inside a sentence, corpus speech.
  - Confirmations of the Russian test phrases still take under 2 s on an idle service (the bar of `limit-confirmation-fallback`).
  - `just full-check` shows no new failures.

## Out of scope

- The user's other recordings beyond keeping them recognized; a new recording session.
- A different speech recognition, keyword spotting or voice activity model.
- The prompt (`restore-recognition-prompt`), the retry schedule of confirmations (`limit-confirmation-fallback`), where an utterance ends and where its confirmation clip ends (`fix-short-pause-after-command`).
- The clip of every other confirmation: only the second hearing uses the steady clip.
- Near misses of other commands; new command phrases or short forms.
- How a dictation is transcribed.

## Release

PATCH (bug fix): target version 0.14.4, the next patch after 0.14.3 on `main`; if `main` moved on, the next patch after the version on `main` when this lands.

## Impact

- `command-spotting/src/engine/clip.rs` (new): the steady clip; `command-spotting/src/engine/confirming.rs`: the second hearing of a near miss; `command-spotting/src/matcher.rs`: the near misses ("start" for "restart dictation").
- `command-spotting/tests/engine_it.rs`: the second hearing with a scripted speech recognition service.
- `app/tests/voice_forms_it.rs`: the `KNOWN_WRONG` entry removed.
- `e2e-test/tests/steps/voice_control/session/speech_after_start.rs`: `UNSTEADY_RECORDINGS` removed; `e2e-test/tests/features/voice_control.feature`, its steps and `e2e-test/src/voice_commands.rs`: the new scenarios and the quiet level.
- `docs/ubiquitous-language.md`: *Second hearing*, *Steady clip*.
- `app/Cargo.toml`: version.
- No manual run: hotkey, recording, overlay, paste and permissions do not change.
