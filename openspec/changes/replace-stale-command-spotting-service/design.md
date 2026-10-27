## Context

The supervision adopts any port whose `GET /health` answers; the command spotting service answers `ok` there whatever protocol it speaks. Its protocol version shows only after a WebSocket hello: a service of another version closes the connection with a policy close naming both versions ("the app speaks protocol version 3, the service version 2"), which every released service does, version 2 included. The client sends the hello after it picked a copy, so a refusing copy is picked again on every reconnect.

## Goals / Non-Goals

**Goals:** never adopt or use a copy of another protocol version; replace it without a manual step.

**Non-Goals:** a version field in `/health`, any change to the service or the protocol, the speech recognition services.

## Decisions

1. **The greeting tells the version.** A new function of the command spotting client connects to one port, sends the hello and a ping and waits up to 2 s: a pong means the copy speaks the app's version; a policy close whose reason names "protocol version" means another one, with the service's version read from the reason. It works against every copy already on disk. Rejected: a version in `/health`, since old copies send none and every 0.41.0 copy would be replaced once for nothing.
2. **A protocol check beside the health check.** `ServiceProbe` gains a `protocol` method (default: same version, like `warm_up`), and `ProbeAnswer` stays as it is. The supervision asks it only when it decides to adopt a command spotting copy and when an adopted copy answers again, never on the 5-second checks, so the service log gets no connection every 5 s.
3. **Replace = stop the listener, then start our own.** On another version the supervision logs one line (`services: <label> at port <p> speaks protocol version <old>, the app version <new> — replacing it`) and asks a new launcher method to stop the process listening on the port. The adapter finds it with `lsof -t -iTCP:<port> -sTCP:LISTEN`, stops it only when its executable has the service program's file name, with SIGTERM, then SIGKILL after the stop grace (5 s), and the supervision then starts its own copy as for a closed port. A listener of another name is not touched: the service is Failed with a reason naming the port.
4. **A refusal while voice control runs rechecks the copy.** The client's connection to the copies greets each candidate and skips one that refuses for its version, so voice control is served by the next copy. Each refused port goes to a hook the runtime hands to the command spotting adapter: `ServicesView::recheck(port)`, which sends an adopted (or stopped-outside) unit back to Pending, so decision 2 and 3 run on it. With `manage` off there is no hook and the client only skips.

## Dependencies

`restart-crashed-services` (filed, not started) reworks the same supervision states for adopted copies; it declares it runs after this change, so this change waits for nothing.

## Risks / Trade-offs

- A service started by `just start-model-servers` with `manage` on and another version is stopped; the script restarts its own copy, which races the app for the port. The app then sees a busy or same-version port and adopts or fails as today. Accepted: it needs an outdated checkout beside the app.
- The stop blocks the services thread up to 5 s, as a quit does.
- `lsof` is part of macOS; a missing or failing `lsof` fails the service with a reason, never kills anything.

## Test levels

| Scenario | Level |
|---|---|
| A copy left by an older app is replaced at launch | component `.feature` (supervision with a fake launcher; the probe against an in-test service refusing version 3) + adapter integration (stop of a listener of the program's name) |
| A copy of the same protocol version is adopted as before | component `.feature` |
| Another program refusing the app's protocol is not stopped | component `.feature` + adapter integration (listener of another name left running) |
| A stale copy appears after the app started | component `.feature` (refusal reaches the hook, the unit is replaced) |
| One of two copies speaks an older protocol version | component `.feature` (client against two in-test services) |
