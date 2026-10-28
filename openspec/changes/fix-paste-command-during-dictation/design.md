## Context

"paste text" is missing from the phrases a running dictation accepts, both in the shared command lists (`voice-commands`) and in the app's own list, so the command spotting service is never asked for it during a dictation and the app ignores it if it comes. The dictation does not touch the clipboard until it pastes its own text on stop, so Cmd+V during a dictation pastes what the user copied.

## Decisions

### "paste text" joins the dictation's phrases

`voice-commands` `DICTATING` and the app's `VoiceCommand::DICTATING` both gain "paste text"; the test that checks they agree keeps them in step. During a dictation the pipeline presses Cmd+V through the key presser exactly as in standby (paste sound, paste sign), and leaves the dictation alone: no cut, no stop, no restart. The overlay's sign gives way back to the recording after about a second, as the existing sign requirement already says. Chains that start with "paste" during a dictation are out of scope; `accepted_after` stays as it is.

### The command word is removed from the dictated text, not from its audio

The recording keeps going across the command, so its words land in the middle of the transcript, where the existing cut (`truncate_at`, used by commands that end the dictation) cannot reach. The dictation counts the "paste" commands carried out during it; when it ends, its text loses up to that many "paste" or "paste text" (with an optional "please") that close a sentence or the text, since a command counts only on its own or ending an utterance. A "paste" inside a sentence ("paste the link") is no command and stays. The removal lives in the domain next to the other command-word removals (`command_words`), and runs in `Dictation::without_command_words`, which both the live and the whole-recording paths already call.

Rejected: cutting the command's audio out of the recording — the recorder can only cut its end, and the live chunks may already be sent; removing every "paste" — it would eat ordinary speech.

## Test levels

| Scenario | Level |
|---|---|
| Paste during a dictation is carried out | component `.feature` (`app/tests/features/component/voice_control.feature`), replaces "Paste during a dictation is ignored" |
| Saying paste during a dictation pastes the clipboard and the dictation goes on | component `.feature` |
| A dictation with a paste in it pastes its text without the command | component `.feature`; unit tests of the removal in `command_words` |
| Stop dictation without a dictation is ignored | component `.feature` and E2E `.feature`, unchanged |
| Clear during a dictation is ignored | component `.feature`, unchanged |
| Start or let's start on its own during a dictation is ignored | E2E `.feature`, unchanged |
| Start ending a dictated sentence is ignored | E2E `.feature`, unchanged |
| Start inside a dictated sentence is ignored | E2E `.feature`, unchanged |
| Saying paste text pastes the clipboard at the cursor | component `.feature` and E2E `.feature`, unchanged |
| Paste leaves the clipboard as it was | component `.feature`, unchanged |
| A paste phrase inside a sentence is ignored | E2E `.feature`, unchanged |
| Clipboard text really lands in the focused app during a dictation | manual run (paste area) |

## Risks

- "…copy and paste." ending a dictated sentence is taken for the command, as "stop" ending a sentence already is: the clipboard is pasted and the word dropped. Accepted, consistent with the other commands.
