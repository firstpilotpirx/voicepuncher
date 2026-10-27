## Context

The supervision adopts any port whose `GET /health` answers; the command spotting service answers `ok` there whatever protocol it speaks. Its protocol version shows only after a WebSocket hello: a service of another version closes the connection with a policy close naming both versions ("the app speaks protocol version 3, the service version 2"), which every released service does, version 2 included. The client sends the hello after it picked a copy, so a refusing copy is picked again on every reconnect.

## Goals / Non-Goals

**Goals:** never adopt or use a copy of another protocol version; replace it without a manual step.

**Non-Goals:** a version field in `/health`, any change to the service or the protocol, the speech recognition services.

## Decisions

1. **The greeting tells the version.** A new function of the command spotting client connects to one port, sends the hello and a ping and waits up to 2 s: a pong means the copy speaks the app's version; a policy close whose reason names "protocol version" means another one, with the service's version read from the reason. It works against every copy already on disk. Rejected: a version in `/health`, since old copies send none and every 0.41.0 copy would be replaced once for nothing.
2. **A protocol check inside the one decision.** `ServiceProbe` gains a `protocol` method (default: same version, like `warm_up`), and `ProbeAnswer` stays as it is. `Supervision::decide`, the single adopt-or-start decision after `restart-crashed-services`, asks it before it adopts a `Ready` command spotting copy; the 5-second checks never ask, so the service log gets no connection every 5 s.
3. **Replace = stop the holder, then decide again.** On another version the supervision logs one line (`services: <label> at port <p> speaks protocol version <old>, the app version <new> — replacing it`), calls `ServiceLauncher::stop_holder` from `restart-crashed-services` (it kills the port's listener only when its command line names the service program) and returns to the decision, where the now closed port starts the app's own copy. A foreign holder is not touched: the service is Failed with a reason naming the port and the program. Rejected: a stop of our own beside `stop_holder`, which would be the same `lsof` lookup twice.
4. **A refusal while voice control runs rechecks the copy.** The client's connection to the copies greets each candidate and skips one that refuses for its version, so voice control is served by the next copy. Each refused port goes to a hook the runtime hands to the command spotting adapter: `ServicesView::recheck(port)`, which sends an adopted unit back to the decision, so 2 and 3 run on it. With `manage` off there is no hook and the client only skips.

## Dependencies

Waits for `restart-crashed-services`: it adds `ServiceLauncher::stop_holder` and makes `decide` the single adopt-or-start decision (removing `StoppedOutside`); this change's replacement calls that method from that decision.

## Risks / Trade-offs

- A service started by `just start-model-servers` with `manage` on and another version is stopped; the script restarts its own copy, which races the app for the port. The app then sees a busy or same-version port and adopts or fails as today. Accepted: it needs an outdated checkout beside the app.
- The greeting blocks the services thread up to 2 s per copy at adoption.

## Test levels

| Scenario | Level |
|---|---|
| A copy left by an older app is replaced at launch | component `.feature` (supervision with a fake launcher; the probe against an in-test service refusing version 3) |
| A copy of the same protocol version is adopted as before | component `.feature` |
| Another program refusing the app's protocol is not stopped | component `.feature` |
| A stale copy appears after the app started | component `.feature` (refusal reaches the hook, the unit is replaced) |
| One of two copies speaks an older protocol version | component `.feature` (client against two in-test services) |
