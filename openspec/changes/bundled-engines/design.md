# Design

## Context

Homebrew's `whisper-server` links `/opt/homebrew/opt/ggml/lib/libggml.0.dylib` and `libggml-base.0.dylib` by absolute path and `libwhisper.1.dylib` through `@loader_path/../lib`; Homebrew's ggml 0.27.0 loads its backends (`libggml-metal`, `libggml-cpu`, `libggml-blas`) from its own folder at run time; no `.metallib` is on disk (the Metal library is embedded). llama.cpp publishes macOS arm64 release zips; the command spotting binary is self-contained. `app-bundle-and-dmg` signs the bundle with a self-signed identity; `local-services-runtime` resolves engines from the bundle first.

## Decisions

**D1 — One folder per engine, libraries beside the binary, `@rpath` + `@loader_path`.** whisper's and llama's ggml differ in version but share file names; separate folders prevent cross-loading. `@loader_path` makes the folders relocatable; `-id` on every dylib removes the last absolute path. Alternative: a shared `Frameworks/` folder with one ggml — rejected, the two engines pin different ggml builds.

**D2 — Pinned by a lock file with checksums, never built.** The user's constraint. Homebrew supplies whisper (no official binary exists), the GitHub release supplies llama, the workspace supplies the command spotting service; `just engines` verifies versions and sha256 against `engines.lock` and refuses anything else, so a bundle is reproducible to the byte for a given lock.

**D3 — Relocate first, sign inside-out last, verify strictly.** `install_name_tool` breaks signatures; so the order is copy → relocate → sign libraries → sign binaries → sign the app with `--force` → `codesign --verify --deep --strict`. An `otool -L` sweep is the gate against a leftover absolute path.

**D4 — A smoke run proves Metal and quarantine.** The bundle is exercised from a quarantined copy on the test ports; the engine logs must show `ggml_metal_init`. This catches a CPU-only fallback (a missing backend dylib) and a Gatekeeper refusal of a nested binary, which no unit test can.

**D5 — The resolver stays in `local-services-runtime`'s catalog; this change only adds the bundle lookup and version logging.** No new port: the engine path is configuration data.

**D6 — Licenses ship in `Contents/Resources/licenses/`.** MIT and Apache-2.0 require the notice; copying the files is the simplest compliant form.

## Risks / Trade-offs

- [Homebrew rebuilds a bottle with the same version and new checksums] → `just engines` refuses; the lock is updated deliberately with the reason.
- [Gatekeeper blocks a nested binary on a user's Mac after Open Anyway] → the quarantined smoke run reproduces the state; the README's Open Anyway step covers the app bundle as a whole.
- [llama's zip layout changes between tags] → the recipe finds files by name; the tag is pinned.
- [Bundle size] → about 60–120 MB; measured and documented.

## Test levels

| Scenario | Level |
|---|---|
| Lock parsing, version comparison, relocation list (which references to change) | unit tests of the small Rust helper only if one exists; otherwise the recipe's own checks (`just engines` exits non-zero on a wrong version or checksum: a `just`-level test in `full-check`'s `bundle` stage with a temporary bad lock) |
| No absolute paths; signature verifies; each engine runs with a clean environment | `bundle` stage of `full-check` (recipe assertions) |
| Bundled engines serve a transcription and a refinement with Metal from a quarantined copy | E2E `app_distribution.feature` `@bundle` (`just bundle-smoke`, under the lock) |
| The resolver prefers the bundle folder and logs origin and version | unit tests in `localservices/catalog` (fake `current_exe` path) |
| A DMG runs on a Mac without Homebrew | manual run by the user |
