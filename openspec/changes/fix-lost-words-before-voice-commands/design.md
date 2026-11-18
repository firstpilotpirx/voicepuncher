## Context

**Bug 1.** The service times a command in seconds of the audio it heard; the app turns them into instants by counting samples from the moment its kept microphone opened. The recorder cuts a dictation by the clock. The two agree only while the sample count matches the clock; it lagged about 0.8 s that day (the first sample comes after the moment taken as the start, and a microphone kept open for hours drifts). The dictation of 15:26:26 starts exactly at the end of "start" and is as long as the real span, yet ends 0.81 s early: only the cut is off. The stream already maps sample times to capture times for the report's end (`HeardAudio::captured_at`); the command's start stayed unmapped.

**Bug 2.** Russian words are respelled into command words by letter patterns. The "enter" pattern takes an optional «л» before any of «эеия», so «линтр» reads as "enter" (all 5 such history reports were false). A last word is reported 0.2 s after it is decoded while the recognizer may still be writing it: «интер» grew into «интернет», «интервью» (11 false reports). Yet «интер» is the user's most common real "enter" after «энтер» (245 reports).

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Which clock the cut uses | the stream maps the command's start to its capture time, as it does the report's end; the lead-in keeps reading the command's end by sample position | holds whatever made the stream lag | a fixed 0.8 s margin; counting from the first callback (one cause only) |
| What the event carries | `CommandsSaid.start` becomes the capture time; `end` stays | `start` feeds only the cut; `end` feeds the lead-in and the replay, read by sample position | a second start field |
| «линтр» | the «л»-opened "enter" takes only «э» or «е» after «л» | every «ли…» report was false; «лентер», «лентарентор» still count | dropping «л» |
| A last word that may grow | a last word respelled as one "enter" from a Russian word opening with «ин» waits: it counts once a word follows it, at the gate's pause, or at the close | keeps «…учесть интер» + pause, drops «интер» → «интернет» | a list of longer words; waiting on every last word |

A flag on `TimedWord`, set by `timed_words`, carries the wait to the at-once rule, beside its check for command words that may grow ("start" of "restart"). Only «интер» at the very end of speech waits, for the gate's pause (about 0.3 s after the word, within the 0.6 s bound); «энтер» is unchanged. The glossary entry "Streaming recognizer" gets this exception.

## Risks

- A block's clock time is taken when the stream thread reads it, tens of ms after capture: the cut may fall that much late; the command words are removed from the text as before.

## Test levels

| Scenario | Level |
|---|---|
| A longer word opening with the sounds of enter is no command | unit (`decide`, words of 16:31) |
| A word holding the sounds of enter inside it is no command | unit (respelling, `decide`) |
| Enter written as the opening of longer words still counts at the end of speech | unit (`decide` at the pause and the close) |
| Enter written as «интер» at the end of speech counts at the pause | unit (`decide` at the pause) |
| A short phrase right before enter is pasted | adapter integration (`commandspotting_command_times`) |
| The words right before enter are kept after a long dictation | adapter integration (same file) |
| A command phrase spoken on its own is recognized | E2E (unchanged) |
| A command said after a long time of listening is recognized | E2E (unchanged) |
| A command word inside a sentence counts while listening | E2E (unchanged) |
| Ordinary speech triggers no command | E2E (unchanged) |
| A stop word inside a dictated sentence ends the dictation at once | component .feature (unchanged) |
| Stop right after the last dictated word | component .feature (unchanged) |
| Enter right after other speech | E2E (unchanged) |
| Stop after a long dictation counts without waiting for a pause | E2E (unchanged) |
| A command word followed by a repeat of the words before it is not pasted | unit (unchanged) |
| Start said alone is reported within its bound | E2E (unchanged) |
| Stop said alone is reported at once | E2E (unchanged) |
| Stop opening a sentence counts at once | E2E (unchanged) |
| A Russian sentence heard as opening with stop is no command | E2E (unchanged) |
| Press enter after a sentence is reported without waiting for the pause | E2E (unchanged) |
| The user's recorded commands are reported within their bound | E2E (unchanged) |
| A command after speech in the user's recordings is reported soon after its word | E2E (unchanged) |
