## Why

The services are programs the app does not build: `whisper-server` (whisper.cpp, installed by Homebrew as `whisper-cpp`, today 1.9.5, linked against Homebrew's `ggml` 0.27.0 at absolute paths `/opt/homebrew/opt/ggml/lib/libggml.0.dylib` and `libggml-base.0.dylib`), `llama-server` (llama.cpp, Homebrew `llama.cpp`, with its `libllama`, `libmtmd` and `libggml*` dylibs) and `voicepuncher-command-spotting` (this repository's own crate, which links sherpa-onnx statically — `otool -L` shows only system libraries). A user who installs the DMG has no Homebrew and must not need one (the user's decision: one DMG file, nothing else to install), and the engines are never built from source, not even as a fallback (the user's decision: ready binaries, pinned). `app-bundle-and-dmg` makes the bundle without engines; `local-services-runtime` already resolves an engine first from `Contents/MacOS/engines/NAME/NAME` inside the bundle, then `$PATH`. This change fills that folder.

## What Changes

- The bundle carries every engine in a folder of its own with the libraries it needs beside it: `Contents/MacOS/engines/whisper/` (`whisper-server`, `libwhisper.1.dylib`, `libggml.0.dylib`, `libggml-base.0.dylib`, and every other `libggml-*.0.dylib` of Homebrew's ggml — `libggml-cpu`, `libggml-metal`, `libggml-blas` — because ggml loads its backends at run time from its own folder), `Contents/MacOS/engines/llama/` (`llama-server`, `libllama.dylib`, `libmtmd.dylib`, its `libggml*.dylib` set, and any `.metallib` or `ggml-metal.metal` the release ships), `Contents/MacOS/engines/command-spotting/` (`voicepuncher-command-spotting` from this workspace's release build). Separate folders because whisper's and llama's ggml are different versions with the same file names; sharing one folder would make one of them load the other's library.
- Every binary and library is relocated: `install_name_tool -change` turns each absolute `/opt/homebrew/...` reference into `@rpath/NAME.dylib`, `-id @rpath/NAME.dylib` is set on each library, and `-add_rpath @loader_path` on each binary and library, so an engine finds its libraries in its own folder wherever the bundle lives. `otool -L` on every file must show nothing under `/opt/homebrew` or `/usr/local` afterwards (the `bundle` recipe fails otherwise).
- The engines come from pinned sources, recorded in `app/packaging/engines.lock` (a TOML file: for each engine the source, version and the sha256 of every file copied) — never built from source:
  - whisper: copied from the Homebrew installation on the developer's Mac; `just engines` checks `brew list --versions whisper-cpp ggml` against the lock's versions (1.9.5 and 0.27.0 at filing; corrected at implementation to the pinned values) and refuses to continue with a message naming the versions found and `brew pin whisper-cpp ggml` when they differ; the copied files' sha256 are compared with the lock.
  - llama: the official release archive `llama-TAG-bin-macos-arm64.tar.gz` (a zip at filing; llama.cpp now publishes `.tar.gz`) from `https://github.com/ggml-org/llama.cpp/releases/download/TAG/` (the tag chosen at implementation: the build Homebrew's `llama.cpp` formula on the developer's Mac corresponds to, written into the lock with the zip's sha256), downloaded by `just engines` into `~/.cache/voicepuncher/engines/llama/TAG/` once, its sha256 verified, unzipped; the files are taken from its `build/bin/` (or wherever the zip keeps them — the recipe finds `llama-server` inside).
  - command spotting: `cargo build --release -p voicepuncher-command-spotting`; its version is the workspace's.
- `just engines` stages the three folders into `app/packaging/engines/` (gitignored), relocates them, and prints each engine's version (`whisper-server --help` for the banner, `llama-server --version`, `voicepuncher-command-spotting --version` — the crate gets `--version` from clap) with `PATH=/usr/bin:/bin` and no `DYLD_*` variable, proving each runs on its own. `just bundle` (from `app-bundle-and-dmg`) runs `just engines` when the staging folder is missing or older than the lock, copies the staging folder into `Contents/MacOS/engines/`, signs every library and binary inside it with the self-signed identity (inside-out: libraries, then binaries), then signs the app again with `--force` (a second signing replaces the seal; `--deep --strict` verification must pass), and runs the same `--version` checks on the copies inside the bundle.
- Metal is confirmed, not assumed: the `bundle-smoke` stage (new, in `full-check`, under the machine-wide lock) starts the bundled `whisper-server` on a test port with the dictation model and the bundled `llama-server` with the refinement model, from a copy of the bundle in a temporary folder with the quarantine attribute set (`xattr -w com.apple.quarantine …`, the state a downloaded DMG leaves), waits for readiness, transcribes one corpus recording through the copy-pool client and refines one sentence, checks each engine's log mentions `ggml_metal_init` (Metal in use, not the CPU fallback), and stops them. It runs on the test ports (`VOICEPUNCHER_TEST_*_PORT`) and starts the command spotting service too, checking its `/health`.
- Licenses: `Contents/Resources/licenses/` gets `whisper.cpp-LICENSE` (MIT), `ggml-LICENSE` (MIT), `llama.cpp-LICENSE` (MIT), `sherpa-onnx-LICENSE` (Apache-2.0), `onnxruntime-LICENSE` (MIT), copied by `just engines` from the Homebrew formula folders, the zip and the cargo registry; README § Licenses lists them.
- The resolver of `local-services-runtime` finds the bundle folder from `std::env::current_exe()` (`…/Contents/MacOS/voicepuncher` → `…/Contents/MacOS/engines/NAME/NAME`); unbundled (cargo run) it falls through to `$PATH` as today. The app's log names the engine it uses and its version at every start (`services: whisper-server 1.9.5 from the bundle`).
- Docs: README § Install (nothing else to install; the engines and their versions), § Developer (Homebrew `whisper-cpp` and `just engines` for developers; how to bump an engine: change the lock, run `just engines`, run `just full-check`), § Licenses; `docs/architecture/overview.md` (engine distribution rows), `docs/project-map.md` (`app/packaging/engines/`, `engines.lock`), `docs/ubiquitous-language.md` (Engine, Engine folder, Engine lock). Tech matrix (agent-kit PR, section "macOS Desktop"): engine distribution — prebuilt binaries bundled in a folder per engine with relocated dylibs (`install_name_tool`, `@rpath` + `@loader_path`), pinned by version and sha256 in a lock file, signed inside-out; never built from source, no Homebrew for users.
- The release DMG grows by the engines (about 60–120 MB in total; measured and written into the README at implementation).

Current stage: PoC. Target stage: Prototype (a DMG that runs with nothing else installed).
Scope: the three engine folders, relocation, the lock file, `just engines`, the `bundle` recipe additions, signing, `bundle-smoke`, licenses, resolver bundle lookup, docs, tech matrix.
Acceptance bar: on a Mac without Homebrew (or with `PATH=/usr/bin:/bin` and Homebrew's folders renamed for the test), the DMG's app starts all eight service processes from its own folder, dictation and refinement work, the engine logs show Metal; `otool -L` of every bundled file shows no `/opt/homebrew`; `codesign --verify --deep --strict` passes; `bundle-smoke` passes in `full-check`; `just engines` refuses a wrong Homebrew version and a wrong zip checksum; the user confirms the DMG on their Mac with Homebrew hidden from `PATH`.
Out of scope: building any engine from source; Intel binaries; updating engines automatically; a shared ggml between whisper and llama; Sparkle or any updater.

## Capabilities

### Modified Capabilities

- `app-distribution`: the bundle carries the engines, pinned and relocated, signed inside-out; the release is self-sufficient; licenses are shipped.
- `local-services`: the services start from the bundle's engines when the app runs as a bundle; the engine and version in use are logged.

## Requirements to specify

1. The bundle runs its own engines: an installed app starts every service from the engine folders inside the bundle, with no Homebrew and nothing on `$PATH`.
2. Engines are pinned: `just engines` takes each engine from the source and version of the lock file, verifies its checksum, and refuses a different version or checksum, naming what it found.
3. Engines are relocated: no bundled binary or library references a path outside the bundle, and each loads its libraries from its own folder.
4. Engines are signed with the app: every bundled binary and library carries the app's signature, and the bundle verifies strictly.
5. The GPU is used by the bundled engines: the smoke check of the bundle proves Metal initialised in whisper and llama.
6. The engine in use is logged: at each service start the app logs the engine's path origin (bundle or `$PATH`) and version.
7. Licenses ship with the bundle: the licenses of the bundled engines are in the bundle's resources and named in the README.

## Implementation guide

1. **Lock file** `app/packaging/engines.lock`:
   ```toml
   [whisper]
   source = "homebrew"
   formula = "whisper-cpp"
   version = "1.9.5"
   ggml_version = "0.27.0"
   files = { "whisper-server" = "sha256…", "libwhisper.1.dylib" = "…", "libggml.0.dylib" = "…", "libggml-base.0.dylib" = "…", "libggml-cpu.0.dylib" = "…", "libggml-metal.0.dylib" = "…", "libggml-blas.0.dylib" = "…" }

   [llama]
   source = "github-release"
   repo = "ggml-org/llama.cpp"
   tag = "bXXXX"
   asset = "llama-bXXXX-bin-macos-arm64.zip"
   asset_sha256 = "…"

   [command-spotting]
   source = "workspace"
   ```
   The file list of whisper is whatever `otool -L` closure and `ls /opt/homebrew/opt/ggml/lib/libggml-*.0.dylib` give at implementation; write it from the real machine.
2. **`just engines`** (`justfile`, recipe with a small shell body or a `scripts/`-free approach: everything inside the recipe, as the other recipes do): read the lock with `yq`-free parsing (the recipe uses `grep`/`sed`, or a tiny `cargo run -p voicepuncher --bin`? No: keep it shell; the values are simple key = "value" lines), check `brew --prefix whisper-cpp` and `brew list --versions`, copy files, download the llama zip with `curl -L --fail` into the cache and check `shasum -a 256`, `unzip -q` into the cache folder, find `llama-server` and the dylibs with `find`, build the command spotting crate, stage into `app/packaging/engines/{whisper,llama,command-spotting}/`, then relocate: for each file `otool -L FILE | awk` the `/opt/homebrew` and `@rpath`-less references, `install_name_tool -change OLD @rpath/$(basename OLD) FILE`, `install_name_tool -id @rpath/NAME FILE` for dylibs, `install_name_tool -add_rpath @loader_path FILE` (ignore "already exists"); then the `--version` checks with a clean environment (`env -i PATH=/usr/bin:/bin FILE --version`). Copy licenses. Print a table of engine, version, size.
3. **`just bundle` additions** (after cargo-packager, before verification): copy `app/packaging/engines/` → `dist/voicepuncher.app/Contents/MacOS/engines/`; `codesign --force --sign "$ID" --timestamp=none` each dylib then each binary (`find … -type f`), then the app with `--force --deep`; `codesign --verify --deep --strict --verbose=2`; `otool -L` sweep asserting no `/opt/homebrew|/usr/local`; the `--version` checks on the bundled copies.
4. **`bundle-smoke`** stage: `e2e-test` gains a scenario tagged `@bundle` in `e2e-test/tests/features/app_distribution.feature` with steps that copy `dist/voicepuncher.app` to a temporary folder, set the quarantine attribute, start the engines from it on the test ports with the models from the models folder, transcribe `e2e-test/corpus` recording number 1 through the copy pool client (`voicepuncher-copy-pool`), refine one sentence, read the engine logs for `ggml_metal_init`, stop the engines; `just bundle-smoke` runs it under the lock; `full-check` adds it after `bundle`.
5. **Resolver** in `adapters/secondary/localservices/catalog/`: `bundle_engine(name)` from `current_exe()`; log origin and version (parse the banner of `--version` once at start, cached).
6. **`--version` for the command spotting crate** (clap `version` attribute from `CARGO_PKG_VERSION`).
7. **Docs** as listed; the agent-kit tech matrix PR.

Pitfalls: `install_name_tool` invalidates signatures — relocate first, sign last; `codesign --deep` does not sign files under `Contents/MacOS/engines/` reliably in nested folders — sign them explicitly first; a dylib with an `-id` that is still absolute makes `dyld` look at the absolute path even with `@rpath` references in the binary — set `-id` on every dylib; the ggml backends (`libggml-metal`, `libggml-cpu`) are found by `libggml` through `dladdr` of its own path — they must sit in the same folder as `libggml.0.dylib`; llama's zip may name its library folder differently between tags — the recipe searches by file name; a downloaded zip's extracted binaries carry the quarantine attribute — `xattr -d com.apple.quarantine` on the staged copies, or signing fails to run them in the checks; never run the bundled engines from inside `dist/` while cargo-packager rebuilds it; the Metal check reads the engine's own stderr log (`services` logs of `local-services-runtime` or the smoke step's own capture) — whisper prints `ggml_metal_init: found device: Apple M…`, llama prints `ggml_metal_init: …` too; keep the `engines/` staging folder and `dist/` in `.gitignore`; the lock's sha256 for Homebrew files change when Homebrew rebuilds the bottle for the same version — then the lock is updated deliberately, with the reason in the commit.

## Impact

- new `app/packaging/engines.lock`, gitignored `app/packaging/engines/`, `Contents/Resources/licenses/` in the bundle
- `justfile` (`engines`, `bundle` additions, `bundle-smoke`, the `full-check` stage list, `.gitignore`)
- `app/src/adapters/secondary/localservices/catalog/` (bundle lookup, version logging), `command-spotting/src/main.rs` (`--version`)
- `e2e-test/tests/features/app_distribution.feature` (`@bundle`), `e2e-test/tests/steps/app_distribution.rs`, `.config/nextest.toml` (a limit for the bundle suite: twice its longest run, at least 2 min)
- `README.md`, `docs/architecture/overview.md`, `docs/project-map.md`, `docs/ubiquitous-language.md`; agent-kit tech matrix (separate PR)
- `openspec/specs/app-distribution`, `local-services` deltas
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge (with `bundle` and `bundle-smoke`); manual run by the user: install the DMG on their Mac with Homebrew's folders hidden from `PATH` (or on another Mac), launch, dictate, paste the log.

## Open points

- The llama.cpp tag to pin: proposed the build matching the Homebrew `llama.cpp` installed on the developer's Mac at implementation, so the E2E (Homebrew engines) and the bundle (zip) run the same code.
- Whisper from Homebrew (proposed; whisper.cpp publishes no macOS server binary) versus a Homebrew bottle fetched by URL for exact reproducibility (`brew fetch --bottle-tag arm64_sequoia whisper-cpp` gives only the current formula version): proposed the installed files with the version check and the lock's checksums.
- Where the engines live: `Contents/MacOS/engines/` (proposed; the resolver already looks there) versus `Contents/Resources/engines/` (cargo-packager's `resources` could copy them, but signing order is the same).
- Whether `bundle-smoke` belongs in `check` too: proposed no (`full-check` only; it needs the bundle and the lock).

## Release

SemVer: MINOR — the bundle gains the engines (new behaviour of the distribution) and the services run from it; pre-1.0.
Target version: 0.33.0 (main is at 0.32.0; corrected at merge with `kit version-check`).
