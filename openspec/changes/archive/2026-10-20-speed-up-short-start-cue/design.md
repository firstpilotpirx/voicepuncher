# Design

## Context

The app cues within 50 ms of a confirmation; the delay is in the engine (proposal.md § What Changes). The command speech recognition service runs three copies behind one port, so two requests of one connection at once are answered side by side.

## Goals / Non-Goals

**Goals:** remove the three causes; no command is found that is not found today.

**Non-Goals:** the timed checks, the spotted-check delays, the app.

## Decisions

### 1. A start ending in "please" is complete

The matcher's start-at-head rule takes "start (dictation) please" with nothing after it, as it takes "start" followed by a word: "please" closes the command, so the early head check and the first spotted check accept "Start please." at once. The second-round rule for a bare "start" stays as it is: a bare "start" heard alone may still be cut before its next word ("start stop"). Rejected: an earlier head check — it would hear "Start." alone and defer to 1.5 s.

### 2. A trigger in a short segment being heard asks its second hearing at once

When the trigger stream spots "start" or "restart" inside a short segment whose hearing is still pending (or the segment closes over a trigger spotted already), the steady clip of that segment is heard at once, on a lane of its own, instead of after the first answer came back as a near miss. The first hearing wins when it holds a command; otherwise the second one's answer applies, as the near-miss path applies it today. Rejected: the check and segment lanes — the request would queue behind another.

### 3. In standby a short utterance is heard at half the pause

While "start dictation" is listened for, once the quiet run of a running short utterance reaches half the minimum silence, its audio so far (pre-roll included, as the segment would send) is heard at once. The answer stands for the segment's hearing only when the segment then closes with that same quiet run and no louder frame came after it; any louder frame drops it, and the segment is heard as today. Only in standby: during a dictation every utterance would cost an extra request. Rejected: a shorter minimum silence — it splits "start dictation" in two.

## Dependencies

Waits for match-late-confirmation-to-its-state: it makes every job read its answer with the phrases it was asked under, and a re-hearing inherit its job's; the early second hearing and the half-pause hearing added here are such jobs and follow that rule.

## Risks / Trade-offs

- [Extra requests in standby on every short sound followed by half a pause] → only short utterances, one request each; the command service has three copies.
- [A second hearing at once may find "restart" where the first heard something else] → it applies only when the first holds no command, as today; the false-start scenarios guard it.

## Test levels

| Scenario | Level |
|---|---|
| Start please followed by the dictation at once gives its cue at once | unit (matcher), E2E `.feature` `@seams` |
| Start please said on its own gives its cue at once | unit (matcher), E2E `.feature` `@seams` |
| A brief dip inside the command words starts nothing early | adapter integration, `command-spotting/tests/placed_commands_it.rs` (scripted hearing) |
| Restart said on its own gives its cue at once | adapter integration, `placed_commands_it.rs` (scripted hearing: first "Restocked.", second "Restart!"), E2E `@seams` |
| Start said on its own gives its cue at once | adapter integration (heard before the pause ends), E2E `@seams` |
| The user's recorded starts give their cue at once | E2E `.feature` `@seams @voice_samples` |
| Start start followed by the dictation at once gives its cue at once | E2E `@seams`, unchanged |
| Start followed by the dictation at once gives its cue at once | E2E `@seams`, unchanged |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `@seams`, unchanged |
| A faster start keeps the dictation where the command's words end | E2E, unchanged |
| A faster start adds no false start | E2E `@smoke` (ordinary speech), unchanged; adapter integration for standby: `engine_it.rs` (start stop, start inside a sentence), `placed_commands_it.rs` (the dip) |
