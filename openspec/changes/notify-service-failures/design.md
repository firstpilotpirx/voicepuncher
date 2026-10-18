# Design

## Context

Every failure in scope ends today in a log line and `VisualIndicator::hide()`. A refinement fallback (inside `refine_or_raw`) and a lost live chunk (in the live worker) never even reach the pipeline. The overlay is one 174 × 55 pt non-activating window per screen with one text label; it fits about ten characters (`◉ REC`).

## Goals / Non-Goals

**Goals:** one notice per failed action, shown by the overlay, built from facts the pipeline has; the notice outlives the cycle's closing `hide()`.

**Non-Goals:** the command spotting service going away while voice control runs (`show-spotting-away`); macOS Notification Center; new retries.

## Decisions

**D1 — The overlay shows the notice.** `VisualIndicator` gains `show_failure_notice(&FailureNotice)`. The macOS overlay shows a warning sign and the notice's message in a wider window (sized to the text, centred where the overlay sits), then returns to its normal size. It stays non-activating, so focus never moves. Rejected: a macOS notification (own permission, often muted, far from where the user looks).

**D2 — `FailureNotice` is a domain type.** `core/domain/failure_notice.rs` holds what was lost (`DictationLost`, `PartOfDictationLost`, `PastedUnrefined`, `NotPasted`, `RecordingNotStarted`, `CommandFailed(VoiceCommand)`, `VoiceCommandsUnavailable`) and the failing part (`SpeechRecognition`, `TextRefinement`, `CommandSpotting`, `Microphone`, `Accessibility`). `message()` renders the short English text, in the overlay's style, with the remedy when one is known: "start local services" for a service, "check Accessibility permission" for accessibility. The text is a pure function, so unit tests pin every message.

**D3 — The pipeline knows the failing part from the port that failed.** The transcriber → speech recognition, the refiner → text refinement, the command spotter → command spotting, the recorder → microphone. The paster and the key presser fail with the domain error `AccessibilityMissing` when the process is not trusted for accessibility (checked before posting events: without the permission macOS drops the events silently today, so no failure is seen at all); the pipeline downcasts to it, and any other paste or key press error shows the notice without a part.

**D4 — Facts that are hidden today become results.** `refine_or_raw` returns the text together with whether it fell back (service failure or rejected answer); the pass-through refiner used with refinement off never falls back, so it shows no notice. The live worker returns the number of chunks it lost with the texts.

**D5 — The notice replaces the cycle's closing `hide()`.** A cycle keeps at most one pending notice; when several apply, the gravest wins (lost > not pasted > part lost > unrefined), and the log keeps them all. Where the cycle hides the overlay today it shows the pending notice instead. Failures outside a cycle (a voice command, listening that cannot start) show their notice at once.

**D6 — The overlay times the notice.** The adapter hides the notice 4 s after it is shown, with the same generation counter the pulse and heartbeat use: any later show (recording, a stage, another notice) bumps the generation, so a stale timer never hides it. Showing the same notice again restarts the 4 s, so a repeated failure keeps one notice on screen.

## Test levels

| Scenario | Level |
|---|---|
| The notice names the failing service and its remedy | unit (`FailureNotice::message`) + component `failure_notices.feature` (shown within 2 s); focus: adapter integration `visualfeedback_macos_it.rs` + manual run |
| A hotkey dictation is lost; A voice dictation is lost | component `failure_notices.feature` |
| One chunk of a long dictation is lost | component |
| The text refinement service is down; Refinement turned off shows no notice | component |
| Pasting is refused | component; `AccessibilityMissing` from the paster: unit |
| The microphone cannot start | component |
| A key press is refused | component |
| The command spotting service is not running at start | component |
| The notice hides by itself; A new dictation replaces the notice | adapter integration `visualfeedback_macos_it.rs` (timer and generation); component (recording shown after the notice) |
| The same failure twice in a row | component (the same notice shown again, no hide between) + adapter integration (one window, timer restarted) |

## Risks / Trade-offs

- `show-spotting-away`, `confirm-voice-commands` and `show-live-transcript` also add overlay states and text; whichever lands later merges with this one's port method and window sizing.
- Checking accessibility trust before each paste adds a cheap system call per paste; a permission revoked while the app runs now shows a notice instead of pasting nothing.
- Four seconds may be short for the longest message; the manual run checks it reads in time.
