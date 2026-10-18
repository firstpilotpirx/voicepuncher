# Design

## Context

With live chunking the recorder hands chunks to a `live-transcription` thread (`run_live_worker` in `pipeline.rs`) that owns the transcriber and collects chunk texts; the pipeline thread merges them with `merge_chunk_texts` only at the stop. The `VisualIndicator` is a `Box<dyn VisualIndicator>` owned by the pipeline thread, so the worker cannot reach it. The macOS overlay is one 174×55 pt pill per screen (`WindowSet`), rebuilt on every state change and on screen changes, with every AppKit call dispatched to the main queue.

## Goals / Non-Goals

**Goals:** the worker shows the stitched text after every chunk without waiting for the pipeline thread; the pill keeps its look and size.

**Non-Goals:** removing spoken command words from the shown text (the specs ask for the raw transcript); changing chunk size or seam stitching.

## Decisions

1. **A new driven port `LiveTranscriptView`** (`core/application/live_transcript_view.rs`, one method `show_live_transcript(&self, text)`, `Send`), handed out by a new `VisualIndicator::live_transcript_view()` that returns a `Box<dyn LiveTranscriptView>` sharing the indicator's state. The pipeline takes one when it starts live chunking and moves it into the worker. The method has a default returning a view that shows nothing, so test stand-ins of `VisualIndicator` stay as they are. *Rejected:* making `VisualIndicator` `Sync` behind an `Arc` (changes every owner); sending the text back as a `PipelineInput` (the pipeline thread is busy only with inputs, but it would add a round trip and a new input kind for a display concern).
2. **The worker merges after every chunk with text** — `merge_chunk_texts` over the texts so far, then `show_live_transcript`. A chunk without text (hallucination dropped, lost chunk) shows nothing new. Merging every time is quadratic in chunks but a dictation has few chunks of ~30 s; the final merge at the stop is unchanged, so the pasted text is the same.
3. **The overlay draws the text in a second borderless window per screen**, under the pill, 480 pt wide, two lines at 14 pt, part of `WindowSet`. It is shown only in the recording state with a non-empty text. The text lives in the indicator's shared state next to `OverlayState`: `show_recording` clears it, every other state hides the window, and a screen rebuild redraws it. An update that reaches the main queue after the state left recording is dropped. *Rejected:* growing the pill (changes the look the processing-stages spec fixes).
4. **The latest words are cut in code, not by AppKit**: a pure function keeps the longest tail of whole words within a character budget (110) and prefixes `…` when it cut. AppKit's head truncation works for one line only.
5. **Glossary:** *Live transcript* — the stitched raw transcript of the chunks transcribed so far, shown while the user dictates.

## Risks / Trade-offs

- [The character budget overflows two lines with wide letters] → budget picked for 14 pt system font in 480 pt; the label also truncates its tail, so it never grows.
- [A dictation shorter than the chunk threshold (30 s) shows no text] → accepted at explore; the specs state it.

## Test levels

| Scenario | Level |
|---|---|
| A hotkey dictation shows its first part once it is transcribed | component `.feature` |
| The shown text grows with every transcribed part | component `.feature` |
| A dictation started by voice shows its text so far | component `.feature` |
| A short dictation shows the recording sign alone | component `.feature` |
| A part without speech adds no text | component `.feature` |
| A long text shows its latest words | unit (tail cut) + adapter integration (label text) + manual run |
| Stopping a dictation clears its text | component `.feature` (pasted text) + adapter integration (window hidden) |
| Cancelling a dictation clears its text | adapter integration |
| The next dictation starts without text | adapter integration |
