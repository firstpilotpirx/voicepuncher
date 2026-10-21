## Context

The proposal lists the gaps; the code involved: supervisor `core/application/local_services/`, `catalog` (`health_checked = false` for whisper), the dictation client's `RETRY_PAUSES` (~1.4 s), the refinement client's single retry, `command-spotting/src/confirm.rs`, the WebSocket `link.rs`.

## Decisions

**D1 — Health-check every whisper copy the app starts.** `catalog` sets `health_checked = true` for whisper copies; the probe already asks `GET /health` and `GET /`. whisper-server answers `/health` without its inference lock, so only a paused or dead copy fails it.

**D2 — Waiting for a copy is a `copy-pool` call.** `CopyPool::await_copy(wait) -> bool` blocks until a probe round sees a copy accept connections or `wait` passes. The dictation client calls it once per request, after its dropped-connection pauses are used up and only when `has_copies()` is false; on success it resets the dropped counter. A service that accepts and drops keeps today's four attempts. The 30 s wait is a field, short in adapter tests. Rejected: longer retry pauses, which delay every dead-service failure as much.

**D3 — The text refinement and command spotting services as copies.** The `copy-pool` address scheme (port + 10000 + n, single copy on the service's own port) is reused for both: `catalog` builds one spec per copy for llama and command spotting the way it does for whisper (`copy_ports`, `running_copies` for adoption), with `[services] refinement_copies` and `command_spotting_copies` (default 2, clamped 1–4). The refinement client leases from a `CopyPool` of its base URL (protocol-free; one request per copy matches llama's `-np 1`). A second Qwen3-4B copy costs ~2.5 GB: accepted, refinement is off by default.

**D4 — Refinement retry.** One more attempt after a refused or dropped connection or a timeout, on another copy (`lease_avoiding(tried)`); before it, when no copy accepts connections, `await_copy(RECOVERY_WAIT)`. A hung copy ends at the request timeout or when the supervisor kills it after 15 s; both lead to the retry.

**D5 — The WebSocket client tries every copy.** `SpottingClient::connect` takes the service URL and tries the service's own port, then its copy ports (`ServiceAddress::copy_ports`), keeping the first handshake that completes. Reconnecting uses it too, so the first attempt after a loss (250 ms) lands on another copy and the 5 s replay covers the gap.

**D6 — Confirmation failover.** `UtteranceTranscriber::transcribe` loops while the deadline lasts: a refused attempt marks the copy down (already) and leases again avoiding the tried copies; other failures end as today.

**D7 — E2E runs the app's own supervisor.** The new `@resilience` scenarios start `ManagedServices` in-process on scenario ports with a home of their own, then drive the adapters (transcriber, refiner, hands-free session) while killing or pausing service processes found by port; few copies keep the light lock class light.

## Test levels

| Scenario | Level |
|---|---|
| An owned service exits | unit (supervision, existing) |
| The pause grows while a service keeps exiting | unit (supervision, existing) |
| An owned service that stops answering is restarted | unit (supervision, existing) |
| An owned speech recognition copy that stops answering is restarted | unit (catalog health-checked) + E2E `local_services.feature` |
| Refinement on starts two copies of the text refinement service | unit (catalog) |
| Voice control starts two copies of the command spotting service | unit (catalog) |
| One configured copy runs on the service's own port | unit (catalog) |
| A text refinement service the scripts run is adopted | unit (catalog) |
| Every copy of the dictation service is killed while a chunk is sent | E2E `speech_recognition.feature` |
| The service does not come back | adapter integration `transcription_openai_it.rs` |
| The text refinement service drops one connection | adapter integration (existing) |
| The text refinement service keeps dropping the connection | adapter integration (existing) |
| A copy of the text refinement service is killed during a refinement | E2E `text_refinement.feature` |
| Every copy is down while the app restarts them | adapter integration `textrefinement_openai_it.rs` |
| A copy that sends no answer is replaced by another | adapter integration `textrefinement_openai_it.rs` |
| The command spotting copy the app listens with is killed | E2E `voice_control.feature` |
| A copy for commands is down as a confirmation reaches it | integration in `command-spotting` (`confirm.rs` tests) |
| A copy for commands is killed while voice control runs | E2E `voice_control.feature` |
| `CopyPool::await_copy` | unit (`copy-pool`) |
| `SpottingClient::connect` tries every copy | adapter integration `commandspotting_reconnect_it.rs` |
