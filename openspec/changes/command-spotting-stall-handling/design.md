## Decisions

### A copy's silence is a failure, not an answer
`UtteranceTranscriber::transcribe` treats a copy that sends nothing within the decoding budget like a refused copy: the port joins the tried list and the confirmation goes to another copy (or the same one, once it is free again) while the deadline lasts; when the deadline runs out the confirmation fails. The worker's second attempt of a segment then runs as designed. An empty answer can now only mean the copy heard no words.

### Failed segment → `confirmation_failed`
When a segment's confirmation ends without an answer, the engine sends `confirmation_failed {start, end}` through the outbox after the segment's own events, so stream order holds. Checks, tails and rehearings that fail send nothing: the segment's own confirmation decides. The app maps it to a new `SpottingEvent::ConfirmationFailed`, which the pipeline answers with the failure notice "Voice commands unavailable" shown at once; the recognizer ignores it. The worst case (two attempts of a 3 s deadline) reports within ~6 s, inside the 10 s bar.

### Stall = a confirmation running past its attempts
Each worker thread records when it took its current job. A job running longer than `max(10 s, attempts × deadline + 2 s)` cannot be a slow copy (every request is given up at its deadline): the thread itself hangs. The processing thread passes that age to its `Beat`; `Health::stuck` counts it like stuck audio, so `/health` answers 503 and the app's supervision restarts the copy. The processing thread also sends `stalled {waited_ms}` once per stall; the app's client turns it into a connection error, so the adapter's outage path (reconnect to another copy, replay) runs. Outbox depth is logged in the connection's stats, not used as a trigger: a deep outbox behind a slow copy is bounded by the deadlines.

Rejected: the age of the oldest unanswered job as the trigger — jobs queued behind a slow copy age past 10 s without anything hanging, and the existing requirement "A slow command spotting service does not stop listening" forbids reacting to that.

### Bounded queues
The I/O thread counts the samples waiting for the processing thread. Audio that would take the backlog past 160 000 samples (10 s) is dropped on arrival, and the I/O thread sends `dropped {samples}` itself. Dropping the newest keeps the queue a plain channel; either end breaks the stream clock, so the app treats `dropped` as a loss and reconnects with its replay, which restores a consistent clock. Each worker queue takes at most 4 unanswered jobs; past that a job is answered at once with no hearing and logged, and a failed send is logged instead of swallowed.

### Protocol version 2
`AppMessage` becomes internally tagged by `type` (`hello`, `select`, `sync`). The first text message must be `{"type":"hello","version":2}`; anything else first, or another version, closes with a policy close naming both versions. A message that does not parse closes with `invalid data` and the parse error; a binary frame whose length is not a multiple of 4 closes the same way. The app sends `hello` in `SpottingClient::connect`.

### `SpottedChecks` holds a list
`waiting` becomes a list ordered by due position; `spotted` adds a first-round check only when none waits, `found_nothing` adds its second round, `take_due` takes the earliest due one.

### Lazy copy discovery
`CopyPool::discover` spawns the probe thread and returns; the thread runs the first round at once. `has_copies`, `snapshot`, `lease*` and `await_copy` wait on the pool's condvar for that first round (at most the probe's own time). `is_same` is removed.

### Size budgets
Files listed in `app/tests/size_baseline.toml` that this change touches (`confirm.rs`, `worker.rs`, `engine/confirming.rs`, the app's `client.rs`, `websocket/mod.rs`, `pipeline/voice_control.rs`, `tests/steps/voice_control.rs`) and the `command-spotting/src` directory make room by moving their unit tests to sibling `tests.rs` files before code is added.

## Test levels

| Scenario | Level |
|---|---|
| One copy for commands stalls mid-command | unit (`confirm` tests) |
| Every copy for commands stalls | unit (`confirm` tests) |
| The service reports a segment it could not confirm | adapter integration (`command-spotting/tests/stall_it.rs`) |
| The app shows a failed confirmation | component (`voice_control.feature`) |
| A flood of audio past the backlog is dropped | unit (`server` backlog tests) |
| The app connects again after audio was dropped | adapter integration (`app/tests/commandspotting_stall_it.rs`) |
| A client of another protocol version is refused | adapter integration (`command-spotting/tests/server_it.rs`) |
| A malformed message closes the connection with a reason | adapter integration (`server_it.rs`) |
| An audio frame of partial samples closes the connection with a reason | adapter integration (`server_it.rs`) |
| A second check does not replace a check still waiting | unit (`engine/spotted.rs`) |
| A confirmation thread that hangs is seen | unit (`health`, `worker`, `server` tests) |
| The app leaves a stalled service | adapter integration (`commandspotting_stall_it.rs`) |
| A confirmation past the bound fails at once | unit (`worker` tests) |
| A client is created while the copies are slow to probe | unit (`copy-pool` tests) |

No E2E scenario is added: every outcome is observable below E2E, and the existing `@resilience` scenarios keep covering paused and killed services end to end.
