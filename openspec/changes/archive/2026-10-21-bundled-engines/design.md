# Design

## Context

Homebrew's `whisper-server` links `/opt/homebrew/opt/ggml/lib/libggml.0.dylib` and `libggml-base.0.dylib` by absolute path and `libwhisper.1.dylib` through `@loader_path/../lib`; Homebrew's ggml 0.27.0 loads its backends (`libggml-metal`, `libggml-cpu`, `libggml-blas`) from its own folder at run time; no `.metallib` is on disk (the Metal library is embedded). llama.cpp publishes macOS arm64 release zips; the command spotting binary is self-contained. `app-bundle-and-dmg` signs the bundle with a self-signed identity; `local-services-runtime` resolves engines from the bundle first.

## Decisions

**D1 — One folder per engine, libraries beside the binary, `@rpath` + `@loader_path`.** whisper's and llama's ggml differ in version but share file names; separate folders prevent cross-loading. `@loader_path` makes the folders relocatable; `-id` on every dylib removes the last absolute path. Alternative: a shared `Frameworks/` folder with one ggml — rejected, the two engines pin different ggml builds.

**D2 — Pinned by a lock file with checksums, never built.** The user's constraint. Homebrew supplies whisper (no official binary exists), the GitHub release supplies llama, the workspace supplies the command spotting service; `just engines` verifies versions and sha256 against `engines.lock` and refuses anything else, so a bundle is reproducible to the byte for a given lock.

**D3 — Relocate first, sign inside-out last, verify strictly.** `install_name_tool` breaks signatures; so the order is copy → relocate → sign libraries → sign binaries → sign the app with `--force` → `codesign --verify --deep --strict`. An `otool -L` sweep is the gate against a leftover absolute path.

**D4 — A smoke run proves Metal.** The bundle is exercised from a copy outside the build folder on ports of its own (a quarantined copy: D12); Metal is proven as D10 says. This catches a CPU-only fallback (a missing backend dylib), which no unit test can.

**D5 — The resolver stays in `local-services-runtime`'s catalog; this change only adds the bundle lookup and version logging.** No new port: the engine path is configuration data.

**D6 — Licenses ship in `Contents/Resources/licenses/`.** MIT and Apache-2.0 require the notice; copying the files is the simplest compliant form. ONNX Runtime's license is not on disk (sherpa-onnx's build script downloads its static libraries), so its MIT text is committed at `app/packaging/licenses/onnxruntime-LICENSE`; the others are copied from the Homebrew folders, the release archive and the cargo registry.

**D7 — Values found at implementation.** Homebrew's formula is now `whisper.cpp` (renamed from `whisper-cpp`), 1.9.5, on `ggml` 0.27.0, whose `libggml-base` links Homebrew's `libomp` 23.1.3: the whisper folder carries `libomp.dylib` too, and the lock pins the three formulae. ggml's backends are `libexec/libggml-{blas,metal,cpu-apple_m1,cpu-apple_m2_m3,cpu-apple_m4}.so`, loaded at run time. llama.cpp now publishes `llama-TAG-bin-macos-arm64.tar.gz` (no zip); Homebrew's `llama.cpp` 0.7.0 is commit `620d2225c`, release `b11669`; that archive is already relocatable (`@rpath` + `@loader_path`, backends linked, not loaded) and runs with a clean environment. The lock is TOML with one table per engine and two sub-tables, `<engine>.from` (file → path in its source) and `<engine>.sha256` (file → checksum of the source file), parsed by the recipe's Perl.

**D8 — The compiled-in backend folder is blanked.** Homebrew's `libggml.0.dylib` carries `GGML_BACKEND_DIR` (`…/Cellar/ggml/0.27.0/libexec`) and searches it before the executable's folder; with it, a bundled whisper on a Mac with Homebrew would load Homebrew's backends, and on a Mac without it the folder is merely missing. ggml searches the executable's folder next and, for the backends without variants (`blas`, `metal`), loads `libggml-<name>.so` from the first search path where it exists. Measured: a blanked (all-zero) string makes whisper load BLAS, Metal and the best CPU variant from its own folder whatever the working directory; a path to an existing empty folder (`/var/empty`, `.`) instead loads only the CPU backend. So `just engines` overwrites each `…/Cellar/<formula>/<version>/libexec` string of a staged file with zero bytes of the same length, before signing. Alternative: `GGML_BACKEND_PATH` from the app — rejected, it loads one backend file and needs the app to know ggml's internals.

**D9 — Folder names and version files.** The engine folders are `engines/whisper/`, `engines/llama/` and `engines/command-spotting/` as the proposal names them; the resolver of `local-services-runtime` looked in `engines/<program>/`, so it moves to the folder names. Each folder holds a `VERSION` file written from the lock (whisper `1.9.5`, llama `b11669`, command spotting the crate's version), which the app logs: whisper-server has no `--version`, and running every engine at each launch would cost a process per engine. The engine folders sit in `Contents/MacOS/engines/`, where codesign treats every file as a nested component: `_sign` signs each of them, the `VERSION` files too (codesign keeps a data file's signature in its extended attributes, which `cp -R` and the disk image keep), before the app. An engine from `$PATH` is logged with the version folder of its Homebrew installation (`…/Cellar/<formula>/<version>/bin/…`, after resolving the link), else "version unknown".

**D10 — The bundle engine stage runs the bundled app.** The model stage `e2e-bundle` (heavy lock class) copies `dist/voicepuncher.app` into a temporary folder and launches the copy's `Contents/MacOS/voicepuncher` in hotkey mode with a home of its own, a configuration with refinement and voice control on and ports of its own, `PATH=/usr/bin:/bin`, no `*_BINARY` and no `DYLD_*` variable, and the models from the environment the `justfile` exports — as `local_services`' "A real launch" scenario launches the build. It waits for every service to be ready, transcribes recording 1 of the corpus through the copy pool client, refines one sentence, checks the command spotting service's health, reads the engines' logs in the scenario's logs folder for Metal — whisper logs `ggml_metal` when it sets up the Metal device; llama-server b11669 logs no device at its default verbosity, so the copy's own `llama-server --list-devices`, run with the same clean environment, must list `MTL0` (a missing Metal backend lists none) and stops the app. The stage first runs `just bundle` itself (incremental: seconds after the `bundle` stage), so a `bundle` stage reused from another worktree never leaves it a missing or stale bundle; `just bundle-smoke` runs the `bundle` stage and then it. It starts no shared test service (`_model-stages` skips them for it as for `e2e-resilience`). Its time limit is 2 min: the first measured run took 17 s (the scenario; the stage 21 s with the bundle build).

**D11 — `just engines` is incremental.** It writes the lock's checksum into `app/packaging/engines/.lock-sha256`; `just bundle` restages whisper and llama only when that differs from the lock (or a folder is missing), and always rebuilds and restages the command spotting service (cargo is incremental). The release archive is downloaded once into `~/.cache/voicepuncher/engines/llama/TAG/` (`VOICEPUNCHER_ENGINE_CACHE`); a cached archive with another checksum is refused, never deleted. `VOICEPUNCHER_ENGINES_LOCK` names another lock (the refusal scenarios use it with a staging folder of their own).

**D12 — The automatic stage runs an approved copy; quarantine is the manual check's.** Measured: on a copy of the bundle with `com.apple.quarantine` set (the state a downloaded disk image leaves), every program of the bundle — the app and each engine — is killed at start (exit 137) until the user allows the application with Open Anyway; the bundle is signed with the project's own certificate and not notarized, so Gatekeeper rejects it (`spctl --assess`: rejected). No headless test can give that approval without changing Gatekeeper's settings, so `e2e-bundle` runs a copy without the attribute — the state after Open Anyway, as far as a test can reach it — and the scenario "A user without Homebrew installs the disk image" (the manual run of task 6.2) checks a downloaded disk image after Open Anyway, the nested engines included. This replaces the quarantined copy of D4 and of the proposal.

## Risks / Trade-offs

- [Homebrew rebuilds a bottle with the same version and new checksums] → `just engines` refuses; the lock is updated deliberately with the reason.
- [Gatekeeper blocks a nested binary on a user's Mac after Open Anyway] → no automatic run can reach that state (D12); the manual run of a downloaded disk image checks it, and the README's Open Anyway step covers the app bundle as a whole.
- [llama's zip layout changes between tags] → the recipe finds files by name; the tag is pinned.
- [Bundle size] → about 60–120 MB; measured and documented.

## Test levels

| Scenario | Level |
|---|---|
| Each engine runs on its own from the bundle | E2E `app_distribution.feature` `@bundle` (bundle stage, no lock), on `dist/voicepuncher.app`; also asserted by `_check-bundle` |
| The engine folders hold what the services need | E2E `@bundle` |
| A different Homebrew version is refused | E2E `@bundle`, `just _engines` with a lock and a staging folder of the scenario's own |
| A release archive with another checksum is refused | E2E `@bundle`, same way |
| The pinned engines are staged | E2E `@bundle`, the checkout's lock into a staging folder of the scenario's own |
| No bundled file points outside the bundle | E2E `@bundle`; also asserted by `_check-bundle` |
| Every bundled file carries the application's signature | E2E `@bundle`; also asserted by `_check-bundle` |
| The bundle holds the licenses | E2E `@bundle` |
| A bundle copy serves dictation and refinement on Metal | E2E `@bundle_engines` (`e2e-bundle` stage, heavy lock class; `just bundle-smoke`) |
| A user without Homebrew installs the disk image | manual run by the user (`@wip` scenario) |
| The full check verifies the bundle | E2E `@bundle` (existing, unchanged) |
| The full check runs the bundled engines | E2E `@bundle`, reading `full-check`'s stage list as the existing stage scenarios do |
| The full check fails on a broken bundle | E2E `@bundle` (existing, unchanged) |
| The quick check stays without the bundle | E2E `@bundle` (existing, unchanged) |
| The bundle's engine comes before `$PATH` | unit test in `localservices/catalog/tests.rs` (fake `app_dir`) |
| An app outside a bundle uses `$PATH` | unit test in `localservices/catalog/tests.rs` |
| A bundled engine is logged with its version | unit test in `localservices/catalog/tests.rs` (the origin line the runtime logs) |
| A Homebrew engine is logged with its version | unit test in `localservices/catalog/tests.rs` (a fake Cellar folder and link) |
