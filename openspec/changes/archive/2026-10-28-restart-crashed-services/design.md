# Design

## Context

Each service and each copy has its own supervision, a state machine the runtime's clock drives (`app/src/core/application/local_services/supervision.rs`). An owned copy is already restarted with the 1 → 30 s back-off and marked Failed after five starts that never answered. An adopted copy is only watched: three `Closed` answers move it to `StoppedOutside`, where it waits forever; a `Busy` answer (port taken, no health page) is never counted at all. See proposal.md § Why for the incident.

## Decisions

**1. A stopped adopted copy goes back to the decision, not to a waiting state.** After three `Closed` checks the supervision logs one line (label with the copy, port, reason) and returns to the same decision a launch makes: `Ready` adopts, `Closed` starts the app's own (from then on owned, restarted and stopped at quit), `Busy` waits up to the readiness limit. `ServiceState::StoppedOutside` goes away, and with it the menu's "Stopped outside the app". *Rejected:* a separate replace path beside the decision — two places would decide between adopting and starting.

**2. Every restart passes through the decision too.** A due `Restarting` asks the port before spawning, so a copy the service scripts restarted meanwhile is adopted instead of racing a second start. A start whose process exits while its port answers `Ready` adopts that service and does not count as a failed start. *Rejected:* spawning blindly and letting the bind error count — five such races would mark a healthy service Failed.

**3. A hung adopted copy is stopped through a new launcher method.** `ServiceLauncher` gains `stop_holder(spec) -> HolderStop`: it finds the program listening on the copy's port (`lsof`, the holder lookup `copy-pool` already has, made public as `holder_of`), reads its command line (`ps -o command= -p <pid>`) and kills it (`kill -9`) only when a word of that line names the file of the service's program, so a copy started from another bundle or checkout counts too; otherwise it returns the foreign program's name and pid. The supervision counts three `Busy` checks of an adopted copy (it answered once, since it was adopted), calls `stop_holder`, logs the pid it stopped, then goes to the decision of 1. A foreign holder is logged once and left; the copy stays in the decision's `Busy` wait. Matching the command line, not the `lsof` command name, also covers the fake service programs (shell scripts) the component tests run. *Rejected:* killing whatever holds the port — it may be the user's own program.

**4. Size.** `supervision.rs` is near the 300-line file budget; the adopted-copy watching (decisions 1 and 3) moves to `supervision/adopted.rs`, the restart paths stay.

**Overlap.** `replace-stale-command-spotting-service` adds a protocol check to the same adoption decision. The two changes share a file but neither builds on the other's result; the later merge folds both checks into the one decision.

## Risks / Trade-offs

- [The scripts' supervisor and the app both restart a copy] → decision 2: whichever answers first is adopted; the loser's start exits and is not counted.
- [`lsof`/`ps` slow or missing] → only on the hang path, at most once per three checks; an unreadable holder counts as foreign and is left running.
- [Killing a copy another app run owns] → only when the copy hangs and its program is the service's own; the other run then sees its process exit and decides again.

## Test levels

| Scenario | Level |
|---|---|
| An adopted command spotting copy that stops is replaced | unit (supervision, fakes) + component `.feature` + E2E `.feature` `@resilience` (bug reproduction on real services) |
| An adopted speech recognition copy that stops is replaced | component `.feature` |
| A replaced copy is stopped at quit | component `.feature` |
| A hung adopted copy is killed and replaced | unit + component `.feature` (a process that listens on the copy's port and never answers) |
| Another program on the port is not killed | component `.feature` |
| The service scripts restart a copy first | unit + component `.feature` |
| `holder_of` / command-line match | adapter integration (`app/tests/it/`) |
