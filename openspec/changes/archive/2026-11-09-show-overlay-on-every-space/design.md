## Context

The overlay's windows (`glass/build.rs`) join every desktop (`CanJoinAllSpaces | Stationary | IgnoresCycle | FullScreenAuxiliary`). Measured on the user's machine with the window server's per-window desktop list: a fresh overlay window from this code is on every desktop, stays so after a Dock restart and joins a desktop added later; the running app's windows, built the day before, were on one desktop only, and a recording on the other desktop drew the pill there out of sight. Why the window server dropped them is not reported; setting the same collection behavior again does not move a window, and a test cannot drop a sticky window from a desktop (the window server puts it back), so the fix detects the symptom instead of the cause.

## Decisions

**D1. Check the pill against the current desktop after drawing; rebuild once when it is off.** `Redraw::draw` asks each shown pill window `isOnActiveSpace` after ordering it front; when any is not, it builds the windows again (`do_rebuild`, as for a screen change) and draws the state once more, logging `overlay: pill not on the current desktop, windows built again`. Once per draw, never in a loop: a second miss only logs. Chosen over rebuilding on every show (would break "drawn without rebuilding", 0.05 s) and over toggling the collection behavior (does not move the window, measured).

**D2. A desktop switch while a state shows runs the same check.** The screen watcher also observes `NSWorkspaceActiveSpaceDidChangeNotification`; its handler, on the main queue, does nothing while the overlay is hidden and otherwise runs the check of D1 on the current state. A plain rebuild on every switch was rejected: it rebuilds while hidden, many times a day.

**D3. A test seam stands in for the window server.** As `forget_the_screens` stands in for an unnoticed screen change, `pretend_off_the_current_desktop` makes the next check of D1 report the pill off the desktop; the component steps use it, and a desktop switch is driven through the same handler the notification calls (`desktop_switched`). The real switch is covered by the manual run.

**D4. Old windows after a rebuild stay out of scope.** A rebuilt set's old windows stay in the window server: closing them (`NSWindow.close`) frees nothing, since AppKit itself holds each closed overlay window (about 60 retains, measured in the component harness). Rebuilds are rare (a screen change, a pill dropped from the desktop), so the leak is a few hidden windows per rebuild; finding what holds them is its own change.

## Test levels

| Scenario | Level |
|---|---|
| The recording sign is drawn at once | component (existing) |
| A screen added later gets the overlay | component (existing) |
| The overlay shows on the desktop the user switched to | manual run |
| Windows dropped from the current desktop are built again on show | component |
| A desktop switch during a recording brings the overlay along | component |
