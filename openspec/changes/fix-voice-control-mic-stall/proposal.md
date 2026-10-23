## Why

Voice control silently stops hearing commands while hotkey dictation keeps working. The user says "старт-старт", "enter enter" and nothing happens; only a restart of the app brings voice control back.

Evidence gathered on 2026-10-23 from the user's running app (voicepuncher 0.37.3 from `dist/voicepuncher.app`, pid 21531, started 2026-10-22T18:24:29Z; input device "Wireless Mic Rx", a DJI USB receiver at 48000 Hz):

- `~/.voicepuncher/logs/voicepuncher.log`: voice commands worked until 2026-10-23T08:52:23Z (`voice control: "start dictation"` at 08:52:14Z with `voice start: total=894ms hear=413ms confirm=186ms`, `voice control: "press enter"` at 08:52:23Z). From 10:25:28Z on there are only hotkey cycles (`pipeline recv Pressed` → `command spotting: session=1 listening for [...]` → paste) and not a single `voice control:` line, although the user spoke commands. No `microphone input error`, no lost-service warning, no reconnect in between.
- The command spotting microphone was opened once, at app start (`command spotting: microphone open at 48000 Hz`, 18:24:30Z) and never again.
- The WebSocket to the command spotting service (adopted copy on port 49083, pid 92249) is still established (127.0.0.1:64500 ↔ 49083). Over ~4 s, `nettop` shows the app's bytes out on it growing by ~24 bytes (keep-alive pings only), against 3.5 GB sent before: the app sends no audio.
- `sample` of the app: thread `command-spotting` loops in `stream_to_service` → `SpottingClient::read_events` / `keep_alive`; `stream_audio` takes nothing from the microphone ring. `sample` of the service: `conn-426-io` blocked in the WebSocket read, `conn-426-proc` in `recv_timeout`, CPU 0.1 % — it receives nothing.
- `pmset -g log` shows no sleep or wake in that window; coreaudiod held the Wireless Mic Rx assertion the whole time. Dictation keeps working because every hotkey press opens a fresh input stream.

Likely cause: `WebSocketCommandSpotter::start_listening` (`app/src/adapters/secondary/commandspotting/websocket/mod.rs`) opens `MicInput` once and only keeps it alive (`_capture`); its `CaptureHealth` (`since_last_callback`, `failed`, `app/src/adapters/secondary/recording/cpal/capture.rs`) is never read for this stream. When CoreAudio stops calling the input callback without reporting an error, voice control stays deaf until the app restarts.

Secondary finding: the adopted services lost their log output. The command spotting service has no stdout/stderr open; its log `voicepuncher-command-spotting-49083.log` ends at 2026-10-22T18:08Z, as do `whisper-server-1808{6,7,8}.log`, because the app instance that started them quit and the next one adopted them. The service side of such an outage is therefore unlogged.

## What Changes

- The app notices when the command spotting microphone has delivered no audio for a short time, or reported an error, logs it once, reopens the input device and resumes streaming with the commands currently listened for; it logs the recovery. Voice control comes back without restarting the app.
- Services the app adopts keep a working log, so a later outage can be read on the service side too.

Current stage: PoC. Target stage: PoC.
Scope: the command spotting microphone capture in the app and its recovery; log output of adopted services.
Acceptance bar: a test where the command spotting input stops delivering callbacks without an error shows the app logging the stall and reopening the microphone, and a command spoken afterwards is recognized; the user's manual run with the Wireless Mic Rx shows voice control still working after a long idle period.
Out of scope: the dictation recording stream (it reopens on every press), command recognition accuracy, the service protocol.

## Capabilities

### Modified Capabilities

- `voice-control`: the command spotting microphone recovers from a silent stall or an input error.
- `local-services`: adopted services keep writing their logs.

## Impact

- `app/src/adapters/secondary/commandspotting/websocket/mod.rs`, `app/src/adapters/secondary/recording/cpal/capture.rs`, the service supervision in `app/src/runtime/services` / `app/src/core/application/local_services`.
- Manual run by the user (microphone path), README § Release checklist.

## Open points

- Stall threshold: proposed 2 s without an input callback while listening.
- Whether to reopen the same named device or fall back to the default input when "Wireless Mic Rx" is gone: proposed same device, retry with backoff, log once.
- The root trigger of the CoreAudio stall (USB receiver, macOS 27.0.1) is unknown; the fix recovers from it rather than preventing it.
- Adopted-service logs: proposed to restart an adopted service whose log is lost is too disruptive; instead proposed to have services open their log file themselves. To settle in design.
- Capability names `voice-control` and `local-services` are assumed; correct them to the spec names in `openspec/specs/`.

## Release

SemVer: PATCH — bug fix.
Target version: 0.37.5.
