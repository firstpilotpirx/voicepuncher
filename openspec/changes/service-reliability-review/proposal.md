## Why

Before voicepuncher is called production-ready, a final review of the four local model services: a crashed or hung service must never break a dictation or a voice command the user is in the middle of.

## What Changes

The review found five gaps, each fixed with a test that reproduces it first:

- A hung speech recognition copy is never restarted: the app health-checks every copy.
- With every dictation copy down, a chunk is dropped after 1.4 s: it now waits for a copy to come back.
- Text refinement runs as one instance and a dropped refinement pastes the raw transcript: it runs as copies; a failed refinement goes to another copy or waits for one.
- Command spotting runs as one instance: it runs as copies; listening moves to another at once.
- A refused command confirmation is not sent to another copy.

Current stage: PoC. Target stage: MVP.
Scope: the four services as the app runs them (`[services] manage`): copies, hang detection, retry of in-flight requests.
Acceptance bar: each service killed or paused mid-request still yields the text or the command, and comes back with all its copies; resilience E2E scenarios prove it per service.
Out of scope: the service scripts, models, the GUI.

## Capabilities

### Modified Capabilities

- `local-services`: copies, hung copies
- `speech-recognition`: wait for a copy
- `text-refinement`: copies, retry
- `voice-control`: copies, failover

## Impact

`app/` supervision and service adapters, `copy-pool/`, `command-spotting/`, `@resilience` E2E.

## Decisions

- Refinement and command spotting: 2 copies by default, 1 to 4.
- Hung: three failed health checks, 5 s apart.
- A request waits up to 30 s for a copy to come back.

## Release

SemVer: MINOR (behavior change).
Target version: 0.36.0.
