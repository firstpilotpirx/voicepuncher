## Why

On 2026-10-27 `just update-app` failed at its first step, `git switch main && git pull --ff-only origin main` (exit 128), and left the app neither rebuilt nor relaunched:

![just update-app failing: untracked files of a just-filed change would be overwritten by the fast-forward](assets/56df7a518eeb.webp)

An agent session was filing a change at that moment, and `kit propose` fast-forwarded the main checkout while `git pull` was fetching. Git saw the branch head move under it and refused to overwrite the change's new files. Nothing was lost, but the recipe had stopped, and git's advice, `git reset --hard`, is dangerous here. The user runs `update-app` whenever they like while agents land changes in the background; the two must never break each other.

## What Changes

- `just update-app` no longer fails when another program (`kit propose`, `kit land`) updates the main checkout at the same moment: it waits for that update, or sees the checkout already at `origin/main`, then rebuilds and relaunches.
- When it really cannot update the checkout (local changes, another branch, local commits), it stops with one line: what blocks it and what to do, never `git reset --hard`.
- kit's own update (a fast-forward of a clean checkout on `main`) is safe and stays; no lock is shared with kit.

Current stage: PoC. Target stage: PoC.
Scope: the `update-app` recipe and how it and the agent-side update of the main checkout share that checkout.
Acceptance bar: `just update-app` started while a change is being filed and landed on `main` finishes, rebuilds and relaunches the app with the newest `main`.
Out of scope: the rebuild and the bundle themselves, `cargo watch`, and any change to agent-kit's update of the main checkout.

## Capabilities

### Modified Capabilities

- `change-workflow`: the main checkout is updated safely when the user's `update-app` and an agent's landing run at the same time.

## Impact

- `justfile` (`update-app`) and a new `dev-tools` command that updates the main checkout.

## Decisions

- Where the fix lives: only `update-app`, through a `dev-tools` command; no lock shared with kit (chosen at explore).
- How `update-app` waits: up to about 30 s while another git command holds the checkout, then the clear stop line.
- Which capability holds this: `change-workflow`.

## Release

SemVer: PATCH — bug fix in a developer recipe.
Target version: 0.41.3.
