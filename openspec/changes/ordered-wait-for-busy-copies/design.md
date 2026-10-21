## Context

The scenario "Requests wait their turn when every copy is busy" fails now and then with "the second request was answered before the first one". Two faults add up:

1. **The client loses a waiting request's place.** A request waits for a free copy at most its own time limit (5 s plus half its audio). When that runs out, the transcription client counts a slow attempt, pauses 200 ms and asks again — and the copy pool puts it at the back of the queue. The scenario's long requests (three times the long speech, on two copies sharing one GPU) often outlast that limit; a copy that frees during the pause goes to the second request.
2. **The scenario checks the wrong order.** It compares when the two waiting requests were answered, but they run on two copies that answer at slightly different speeds and free about 200 ms apart, so the answer order can flip with the start order right.

## Decisions

### A request keeps its place across its waits for a copy

The copy pool gets a place in its queue that a client holds for one request: it is taken when the request first asks for a copy, kept when a wait runs out, and given up once a copy is leased or the client drops it. The transcription client holds one place across its attempts and asks again at once (no pause) when only the wait for a copy ran out; the number of attempts and the give-up rule stay as they are. An attempt after a copy failed takes a new place at the back, as today: only waiting keeps the place, so a request that hung a copy does not jump the queue.

Rejected: one longer wait for the first attempt (changes when a request gives up and leaves the same race on the next wait); keeping the original arrival number for every retry (a request that hung one copy would then jump ahead of everyone waiting).

### The scenario checks the start order

The two waiting requests carry audio of different lengths (the same words, the second padded with silence), so each copy's log line for the request it takes names which one it is. The scenario records when each first appears in a copy's log and checks the first arrived starts no later than the second, and that each starts only once a copy freed; the answer-order check goes.

## Test levels

| Scenario | Level |
|---|---|
| Requests that arrive at once are shared by the copies | E2E `.feature` (unchanged) |
| Requests wait their turn when every copy is busy | E2E `.feature` (order check fixed); unit test in `copy-pool` |
| A request whose wait ran out keeps its place | adapter integration (`transcription_openai_it.rs`), unit test in `copy-pool` |
