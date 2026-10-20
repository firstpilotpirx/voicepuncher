## Why

The GUI, the installer and the rest of the open backlog turn voicepuncher into a production-ready app. Before it is called that, the user wants one final review of how reliable the local model services are: the dictation speech recognition service, the command speech recognition service (the small Whisper model), the text refinement service (the LLM) and the command spotting service. Today a crashed service can break a dictation the user is in the middle of; for a production app the user must never notice a service failure.

This change runs last, after every change open on the day it was filed is archived, because those changes (bundled engines, local services runtime, restartable app runtime, whisper copy pool, settings window and others) reshape how services are started and supervised.

## What Changes

- Review, and fix where it falls short, that every local model service runs as several instances behind a load balancer, so one instance failing does not stop the service.
- The app itself detects a crashed or hung service at once and restarts it, without relying on the `just start-model-servers` supervisor or on the user.
- The app reconnects to a restarted service and resends the user's in-flight request (a chunk to transcribe, a text to refine, a command to confirm), so the dictation or command completes as if nothing happened.
- Every gap the review finds is fixed in this change, with a test that reproduces the failure first.

Current stage: PoC. Target stage: MVP.
Scope: the four local model services as the shipped app runs them — instances, load balancing, crash and hang detection, restart, reconnection and retry of user requests.
Acceptance bar: with each service killed or paused during dictation and during voice control, the user's text is pasted and the command runs with no failure notice, and the service is back with all its instances; resilience E2E scenarios prove it for every service.
Out of scope: new features, model or accuracy changes, the GUI's look.

## Capabilities

### Modified Capabilities

- `local-services`: several instances behind a balancer for every service; the app detects and restarts a failed service.
- `speech-recognition`: reconnect and retry of an in-flight chunk after a service restart.
- `text-refinement`: reconnect and retry of an in-flight refinement after a service restart.
- `voice-control`: reconnect to the command spotting service and retry of a command confirmation after a restart.

## Impact

- `app/` service supervision and the HTTP and WebSocket adapters of the four services.
- `command-spotting/`, the `justfile` service recipes and the `@resilience` E2E scenarios.

## Open points

- Whether the text refinement service and the command spotting service also get several instances, given their memory cost; proposed: yes for all four, as the user asked, with the instance count configurable.
- What "hung" means for detection (a timeout per request kind); proposed: design picks it from measured latencies.
- How many retries before a failure notice is shown; proposed: retry until the restarted service answers or a bound is reached, decided in design.
- Target version: depends on what `main` holds when this runs; proposed MINOR above it.

## Release

SemVer: MINOR — the app gains service supervision and request retry, a behavior change.
Target version: 0.25.0.
