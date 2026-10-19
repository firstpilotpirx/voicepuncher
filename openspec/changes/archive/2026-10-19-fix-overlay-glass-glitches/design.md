# Design

## Context

The overlay's glass panels share one builder (`glass.rs`); the pill and the text window are separate borderless windows, each with a 32 pt transparent margin for its shadow. The paster snapshots the clipboard as text only (`arboard::get_text`) and clears it when there is no text. The transcriber sends `language` only when it is fixed and asks for `response_format=json`, which carries no detected language. A probe of the running service: "окей окей" said by `say` came back as Portuguese ("Aqui e aqui."); the same audio with `language=ru` gave "Окей, окей."; `verbose_json` names the detected language (`"language":"portuguese"`).

## Goals / Non-Goals

**Goals:** the nine fixes of the proposal, each proven by a test that fails first.

**Non-Goals:** the fallback material of macOS before 26 beyond dropping the line; a new language-detection model.

## Decisions

1. **Glass edge and transparency.** Drop the top-highlight layer everywhere. On macOS 26 drop our rim too and set the system glass to its clear style (`NSGlassEffectViewStyle::Clear`); the fallback keeps its rim. Rejected: a fainter, gradient line — still a second edge over the system's.

2. **Pill placement.** First an adapter test reads the pill window's real frame after it shows; if it moved from where it was placed, the margin above the glass caused it. The pill window gets no margin above the glass beyond the 4 pt gap (its shadow falls downward), so the window stays inside the visible frame and is not pushed. The text window is placed from the pill's real glass frame each time it shows, never from the rectangle computed at build time. Rejected: subclassing `NSWindow` to skip the frame constraint — more unsafe code for the same result.

3. **Text window only while recording.** `pill_states` stops showing the text window in every state but `Recording`; the refinement preview is dropped from the overlay (the `preview` field goes), the failure notice's message is logged at `warn` when it shows, and the wait-for-focus explanation goes. The live transcript path is unchanged.

4. **One pill size.** `pill_width` returns one fixed width, that of the widest recording row (the full waveform, or the shorter waveform beside the commands-unavailable sign) plus padding, for every kind but the standby commands-unavailable pill; content stays centred. A unit test checks that the widest content of every state (the longest label, a command, a failure part beside the commands-unavailable sign) fits that width.

5. **Slower waveform.** `COLUMN_PERIOD` becomes 200 ms; capacity stays 30 columns, so the strip holds 6 s. The rise and fall smoothing is per column and stays.

6. **Clipboard snapshot.** The macOS paster snapshots every item of the general pasteboard with every type's data (`NSPasteboard` through `objc2-app-kit`) right before it writes the text, and after the paste writes the items back, or clears the pasteboard when it was empty. It restores only when the pasteboard's change count is still the one its own write set, so a copy the user makes after the paste is never overwritten. Rejected: arboard's image API — it covers text and images but drops files and other types.

7. **Allowed languages.** `[transcription] languages` (default `["ru", "en"]`) lists the user's languages; requests ask for `verbose_json`. With `language = "auto"`, a transcript whose detected language is not one of them is requested again with the first language fixed, and that transcript is used; the retry is logged. Whisper names languages in full ("russian"), so a small code-to-name table covers the codes the setting accepts; an unknown code or an empty list fails config loading. Rejected: a fixed `ru` — English terms in Russian speech come out in Cyrillic; a script check on the text — misses a wrong language in the same script.

## Test levels

| Spec scenario | Level |
|---|---|
| visual-feedback: The overlay over a dark window shows light content | manual run (unchanged) |
| visual-feedback: The overlay over a light window shows dark content | manual run (unchanged) |
| visual-feedback: The glass overlay takes no focus | adapter integration (`visualfeedback_macos_it`, unchanged) |
| visual-feedback: The window under the overlay shows through the glass | adapter integration (`visualfeedback_macos_it`: clear style) + manual run |
| visual-feedback: The glass has no bright line along its top edge | adapter integration (`visualfeedback_macos_it`: no highlight layer) + manual run |
| visual-feedback: Speech leaves tall columns that slide to the left | unit (`level_history`, unchanged) |
| visual-feedback: Silence leaves a dotted line | unit (`level_history`, unchanged) |
| visual-feedback: A new recording starts with a dotted waveform | unit (`level_history`, unchanged) |
| visual-feedback: The waveform keeps the last six seconds | unit (`level_history`) |
| visual-feedback: The waveform moves at half its former pace | unit (`level_history`) |
| visual-feedback: The record dot pulses in silence too | adapter integration (`visualfeedback_macos_it`, unchanged) |
| visual-feedback: Held-back text shows the orange wait sign | adapter integration (`visualfeedback_macos_it`) |
| visual-feedback: A pasted dictation shows the Pasted sign | component (`visual_feedback.feature`) + adapter integration (no text window) |
| visual-feedback: Text pasted after a wait for focus shows the Pasted sign | component (`visual_feedback.feature`) |
| visual-feedback: Text left in the clipboard shows no Pasted sign | component (`visual_feedback.feature`, unchanged) |
| visual-feedback: Text pasted without refinement shows the failure notice instead | component (`failure_notices.feature`, unchanged) |
| visual-feedback: A command right after the paste replaces the Pasted sign | component (`visual_feedback.feature`, unchanged) |
| visual-feedback: A lost dictation shows the amber notice with its message | adapter integration (`visualfeedback_macos_it`: no text window, message logged) |
| visual-feedback: A failure notice over a white window stays legible | manual run (unchanged) |
| visual-feedback: The text recognized so far shows below the pill | adapter integration (`visualfeedback_macos_it`, real frames) |
| visual-feedback: Ending the recording takes the text window away | adapter integration (`visualfeedback_macos_it`) |
| visual-feedback: The pill does not change size from the recording to Pasted | unit (`pill_content`) + adapter integration (`visualfeedback_macos_it`) |
| processing-stages: Refinement shows no raw transcript | component (`processing_stages.feature`) + adapter integration |
| text-insertion: A screenshot copied during the recording survives the paste | adapter integration (`textpasting_macos_it`) |
| text-insertion: Copied text survives the paste | adapter integration (`textpasting_macos_it`) |
| text-insertion: An empty clipboard stays empty | adapter integration (`textpasting_macos_it`) |
| speech-recognition: Automatic language detection | E2E (unchanged) |
| speech-recognition: Fixed language | adapter integration (`transcription_openai_it`: `language=en` sent, no retry whatever the service detects) |
| speech-recognition: Short Russian speech is never transcribed as Korean | adapter integration (`transcription_openai_it`, stub service) + E2E (`speech_recognition.feature`) |
| speech-recognition: English speech stays English with automatic detection | adapter integration (`transcription_openai_it`, stub service) + E2E (`speech_recognition.feature`) |

## Risks / Trade-offs

- A retried request costs one more recognition, only when detection drifts.
- The clear glass may lower contrast over busy pictures; the manual run judges it.
- Restoring the pasteboard in a background thread races a fast user copy; the change-count check covers it.
