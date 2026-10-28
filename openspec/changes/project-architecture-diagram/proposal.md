## Why

A person who starts working on voicepuncher has no single picture of the project's structure. The workspace holds many crates (`app`, `command-spotting`, `copy-pool`, `entry-folders`, `log-line`, `voice-commands`, `dev-tools`, `test-steps`, `e2e-test`, `analyzers`), `app/` follows a hexagonal architecture (adapters → core/application → core/domain), and the app talks to three local services (dictation speech recognition, command speech recognition, command spotting) over loopback HTTP and WebSocket. Today this is only described in prose (`AGENTS.md`, `docs/`), so a newcomer has to read a lot of text to learn which packages exist, which layers there are, which components there are and how they interact.

The user wants an HTML page with a diagram of the project — for example a package diagram — so that anyone starting on the project always understands its structure, layers, components and how they interact.

## What Changes

- An HTML document under `docs/` that shows the project's architecture as diagrams:
  - a package diagram of the Cargo workspace: every crate, what it is (binary, library, dev-dependency) and the dependencies between crates;
  - the layers of `app/`: primary and secondary adapters, `core/application` (ports), `core/domain`, and the allowed dependency direction;
  - the main components and how they interact at runtime: the menu bar app, the dictation and command speech recognition services (whisper-server copies and the copy pool), the command spotting service, and the protocols between them (loopback HTTP, WebSocket);
  - a short legend and a one-line explanation of every box, using the terms of `docs/ubiquitous-language.md`.
- A link to the page from the README and the repository layout section of `AGENTS.md`, so a newcomer finds it.
- A test in the change-workflow harness (`dev-tools`) that fails when the page's crate diagram and the Cargo workspace members differ.
- `docs/architecture/overview.md`: its crate table says "four crates" though the workspace has ten; it lists all of them and links to the page.

Current stage: PoC. Target stage: PoC.
Scope: one HTML architecture page (package, layer and component/interaction diagrams) under `docs/`, plus a link to it.
Acceptance bar: the page opens offline in a browser and shows every workspace crate, the app's layers and the runtime components with their connections, matching the code at the commit it lands on; a newcomer can name the crates, layers and services from it alone.
Out of scope: changing the code or the architecture; generating other documentation (ADRs, arc42 chapters).

## Capabilities

### Modified Capabilities

- `change-workflow`: a new requirement — the architecture page names exactly the workspace crates, checked by the project's tests.

## Impact

- `docs/architecture/project-architecture.html` (new), `docs/architecture/overview.md`, `README.md`, `AGENTS.md`.
- `dev-tools/tests/` (the crate check): a build input, so `just check` runs and the branch lands through the landing queue.

## Decisions

- Drawing: by hand with the kit's report diagrams (`kit report`, inline SVG, no network); the crate list is guarded by a test against `cargo metadata` instead of being generated.
- Detail inside `app/`: one box per adapter folder, ports listed in `core/application`; pictures of at most 12 boxes, split by layer.
- Links: README and the repository layout section of `AGENTS.md`.

## Release

SemVer: no release — documentation only, no behavior change.
Target version: 0.41.5.
