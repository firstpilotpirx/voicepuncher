# Proposal

## Why

The user's quiet "restart" does not always start a dictation over: whether it works depends only on the silence in front of it. This breaks two promises of the spec: "restart" said on its own starts the dictation over, and every command in the user's recordings is recognized exactly once, apart from recordings marked unclear (this one is not).

- **The same recording is heard two ways.** In the clip the command spotting service sends to confirm the user's recorded "restart" in `~/.voicepuncher/voice-samples/session-2.wav` (27.23–27.52 s), the speech recognition service hears "Restart." after 0.3, 0.6 or 0.8 s of silence, but "Let's start." after 0.4 or 0.5 s (and "Start." in some E2E runs). That silence is whatever the voice activity detector leaves before the command: anywhere from 0.3 to 0.8 s, depending on the pause before it and on when the detector notices the speech.
- **Not the level, but what comes before the word.** The user speaks quietly, but this "restart" is no quieter than the two other recorded "restart"s in that session, which are heard as "Restart." every time. What sets it apart is about 0.5 s of faint sound right before a weak "re", which after some lengths of silence come out as "Let's".
- **Neither "Let's start." nor "Start." is a command during a dictation.** "let's start" is no command phrase, and "start" is accepted only when no dictation runs. So the restart takes no effect: the dictation goes on with the text the user meant to throw away.
- **The tests leave it out.** `fix-short-pause-after-command` excludes this recording from its E2E scenarios of a recorded command followed by a short pause and speech (`UNSTEADY_RECORDINGS` in `e2e-test/tests/steps/voice_control/session/speech_after_start.rs`) and leaves it to this change.
- **Measured without the prompt.** The command phrases sent as a prompt with every confirmation had no effect when this was measured: whisper-server drops the prompt of every request (`restore-recognition-prompt`). With the prompt in effect, and with the confirmation retries of `limit-confirmation-fallback`, the recording may already be heard right after every pause.

## What Changes

- **Measure again first.** With `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt` on `main`, this recording is heard during a dictation after every silence from 0.3 to 0.8 s, on its own and with speech after it. If it starts the dictation over every time, the change only removes the exclusion and adds the tests below.
- **Otherwise, the confirmation of a short utterance hears a steady clip (the chosen fix).** The clip sent to confirm a short utterance starts a fixed time before the utterance's first sound, whatever pause the user made and whenever the voice activity detector noticed the speech. Only if the measurement needs it, a clip quieter than a normal voice is also raised toward a normal level. The values come from the measurement (design.md). Why this one: only the silence in front of the same words changes what is heard, so the cause is in the clip. A steady clip gives the same words the same answer after any pause, for every short command; it costs no extra request and adds no rule that turns other words into a command.
- **A second hearing only if the steady clip is not enough.** If no steady clip makes this recording heard as "restart" after every pause, a short utterance heard as no command but as a listed near miss of one the state accepts (first "start" or "let's start" during a dictation, where "restart" may have been said) is heard once more with another silence before it, and counts only when that hearing says the command. It is the fallback, not the fix: it adds a request and its delay to every near miss, and it gives a quiet "let's start" a second chance to become a restart.
- **Rejected: taking "let's start" or "start" as "restart" during a dictation.** Both are ordinary words in a dictation, and a restart throws away all that was dictated: a wrong restart loses the user's text, while a missed one only has to be said again. "start" stays no command during a dictation.
- **Tests:** the exclusion is removed, so the scenario "The user's recorded command alone, a short pause, then speech" of `fix-short-pause-after-command` runs on this recording too; a test plays it during a dictation after each silence from 0.3 to 0.8 s; every command phrase and short form said on its own in a quiet voice takes effect; "let's start" and "start" said during a dictation (on their own, ending a sentence and inside one, at a normal level and as quietly as the user speaks) neither restart nor end it.
- No change to which phrases count as commands or to the states that accept them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: the user's own way of saying the commands is recognized whatever the pause before a command, in a quiet voice too; "let's start" and "start" said during a dictation neither restart nor end it.

## Scope

- Current stage: PoC. Target stage: PoC, with the user's quiet "restart" taking effect after any pause.
- In scope: the measurement above; the clip the command spotting service sends to confirm a short utterance; the second hearing, only if the measurement calls for it; removing the exclusion; the tests above.
- Lands after `fix-short-pause-after-command`, `limit-confirmation-fallback` and `restore-recognition-prompt`: each changes what the speech recognition service hears or answers in a confirmation, so the smallest fix can only be measured with all three in place.
- Acceptance bar:
  - The user's recorded "restart" at 27.23 s in `session-2` starts a running dictation over after every silence from 0.3 to 0.8 s, and when 0.3–0.6 s of silence and speech follow it (the scenario of `fix-short-pause-after-command`, without the exclusion).
  - Every recorded command of the user is still recognized exactly once (`voice_samples_it`, `voice_forms_it`), and the E2E voice control scenarios pass.
  - Each command phrase and short form, said on its own as quietly as the user's quietest recorded commands, takes effect in the state that accepts it.
  - "let's start" and "start", said during a dictation on their own, ending a sentence and inside one, at a normal level and as quietly as the user speaks, neither restart nor end it.
  - No command takes effect in the existing negative cases: short Russian phrases, a command word inside a sentence, corpus speech.
  - Confirmations of the Russian test phrases still take under 2 s on an idle service (the bar of `limit-confirmation-fallback`).
  - `just full-check` shows no new failures.

## Out of scope

- The user's other recordings beyond keeping them recognized; a new recording session.
- A different speech recognition, keyword spotting or voice activity model.
- The prompt itself (`restore-recognition-prompt`), the retry schedule of confirmations (`limit-confirmation-fallback`), where an utterance ends and where its confirmation clip ends (`fix-short-pause-after-command`).
- New command phrases or short forms.
- How a dictation is transcribed: only the clip that confirms a command changes.

## Release

PATCH (bug fix): target version 0.13.4, the next patch once the three changes it waits for are released as 0.13.1–0.13.3; if `main` moved on, the next patch after the version on `main` when this lands. If the first measurement shows the recording already works, the change holds only tests, keeps the version, and this section is updated to say so.

## Impact

- `command-spotting/src/engine/confirming.rs` and a new `command-spotting/src/engine/clip.rs`: the clip sent to confirm a short utterance; `command-spotting/src/engine.rs`: its shape in the engine settings.
- `command-spotting/src/matcher.rs`: only for the second hearing, if it is needed: the list of near misses ("start" for "restart").
- `command-spotting/tests/confirmation_sweep_it.rs` (new, run on demand): the measurement; `command-spotting/tests/engine_it.rs`: the clip after each pause.
- `e2e-test/tests/steps/voice_control/session/speech_after_start.rs`: `UNSTEADY_RECORDINGS` removed; `e2e-test/tests/features/voice_control.feature`, its steps and `e2e-test/src/voice_commands.rs`: the new scenarios and the quiet level.
- `docs/ubiquitous-language.md`: *Confirmation clip*.
- `app/Cargo.toml`: version.
- No manual run: hotkey, recording, overlay, paste and permissions do not change.
