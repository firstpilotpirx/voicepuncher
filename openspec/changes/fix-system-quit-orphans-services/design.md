## Context

The app runs an `NSApplication` without a delegate. A quit Apple Event (logout, shutdown, Activity Monitor's Quit, `tell application … to quit`) reaches `-[NSApplication terminate:]`, which, with no delegate to ask, ends the process at once: nothing stops the runtime or the owned services, nothing is logged. Menu Quit goes through `MacosMenuActions::quit` (runtime stop, which stops the owned services, then `exit`); SIGTERM and SIGINT go through `signals::stop_on_termination` (logged, owned services stopped, `exit`). Only the Apple Event path is missing.

## Goals / Non-Goals

**Goals:** a quit from the system takes the menu Quit path and is logged; every quit path is logged.
**Non-Goals:** adopted services (they stay running), the old `haproxy` proxies on 8081/8085, a quit that arrives before the menu's actions are wired (the window is a few milliseconds at launch; SIGTERM is already handled then).

## Decisions

1. **An application delegate that answers "should terminate" with the menu's Quit.** New `app/src/adapters/primary/menubar/macos/system_quit.rs`: a `define_class!` `VPAppDelegate` (`NSApplicationDelegate`) holding the menu's actions (`Rc<dyn MenuActions>`); `applicationShouldTerminate:` logs `quit: asked by the system` and calls `MenuActions::quit`, which stops the runtime and ends the process, so it never returns in practice; when it does return (no actions yet), the reply is `NSTerminateNow`, today's behaviour. `SystemQuit::install(mtm, actions)` sets the delegate and returns the retained object, which `main` keeps for the life of the run loop (`setDelegate` holds it weakly). Alternative rejected: making the status item's `MenuTarget` the delegate — `status_item.rs` is over the file budget and listed, so it may not grow.
2. **Menu Quit is logged too.** `MacosMenuActions::quit` logs `quit: stopping the runtime and the services the app started` before it stops the runtime, so every quit leaves a line (SIGTERM/SIGINT already log `runtime: signal …`). SIGINT keeps the signal path (open point 1: yes, same as SIGTERM). The stop limit is the one Quit uses (5 s, then kill) — open point 2.
3. **`main.rs` does not grow.** `main.rs` and `main` are listed in the size baseline; the three `unwrap_or_else(|e| { log::error!(…); exit(2) })` blocks of the services start, the termination thread and the hang-up thread become one helper `or_exit`, which pays for the delegate's line.

## Risks / Trade-offs

- A logout waits for the reply while the services stop (at most 5 s each, as menu Quit); macOS allows that.
- The regression scenario sends the quit with `NSRunningApplication terminate` through `osascript -l JavaScript` by pid: it needs no Automation permission, since it is the app's own user's process; when the binary is not registered as a running application, `terminate` returns false and the step fails naming it.

## Test levels

| Scenario | Level |
|---|---|
| A quit from the system stops the owned services | E2E (`e2e-test/tests/features/local_services.feature`, `@smoke @serial`, beside "A real launch leaves no service behind at quit") |
| Quit with owned and adopted services, A service that ignores the request to end is killed | unchanged (existing tests) |

In this batch the E2E scenario is not run by this session (batch mode: `just test` and `just lint` only); the batch's `full-check` runs it. Red/green of the delegate is shown by hand with the debug binary: before the fix a quit Apple Event ends the app with no log line and its `whisper-server` copy left; after it the log shows `quit: asked by the system` and the copy is gone.
