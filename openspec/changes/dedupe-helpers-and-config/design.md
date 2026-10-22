## Decisions

### The service command lines: two sources held equal by a test
The proposal's open point proposed the catalog as the only source, with the `justfile` asking the app binary for each command line. Rejected: `start-model-servers` runs inside every locked E2E stage and in `just run`, and would first have to build the whole app in release (minutes on a cold `target/`) only to print a few flags; a bundled app cannot read the `justfile` either. Chosen: the catalog (`whisper_args`, `llama_args`, `command_spotting_args`) stays the app's source and the `justfile` stays the scripts' source, and an adapter integration test (`localservices_command_lines_it.rs`) reads the `justfile`, takes the argument list of each service (the `whisper_args` function, the `llama-server` line of `start_llama`, the line of `start_command_spotting`), puts the catalog's placeholders in for the shell variables and compares it token by token with the catalog. A difference fails with the service's name. The `justfile` feeds every stage key, so a flag changed in it reruns the test.

### One rule for picking a model: newest, then by name
`newest_vad` picked by modification time, `kws_model_dir` by name; the `justfile` picked the VAD with `ls -t` and the KWS folder with `find | head -1` (directory order). Both lookups now use one `newest` helper (latest modification time, ties broken by the name), and the `justfile` picks the KWS folder with `ls -td` like the VAD file.

### Shared helpers live beside what shares them
- `adapters/secondary/http/agent.rs` builds every `ureq` agent: `fresh_connections(timeout)` (no idle connection kept; used by transcription, refinement, the refinement start-up check and the service probe) and `downloads()` (the model fetcher's redirects, user agent and read limits). `HttpProbe` holds one agent instead of building one per probe.
- `adapters/appkit.rs`, next to `main_queue.rs`, holds `titled_window`, `below_top`, `label`, `wrapping` and `push_button` for the permissions and download windows; each window passes its size, title and target. The settings window keeps its builders: its labels are created empty and filled later, so the helpers would not shorten them.

### The probe takes a kind and a port
`ServiceProbe::answer` takes `(ServiceKind, u16)` instead of a whole `ServiceSpec`, so `running_copies` no longer builds a dummy spec; supervision passes its spec's kind and port.

### `spec()` and the three service lookups
The catalog's `whisper`, `llama` and `command_spotting` repeat one shape: parse the address, take the running or configured copy ports, build one spec per port. One `copies` method does that, given a `Service` value (kind, program name, program lookup result, problem) and the per-port argument builder; `spec` takes the `Service` and drops the always-true `health_checked` argument, so it needs no `too_many_arguments` exception.

### Menu entries by identifier
`MenuEntry::place()` returns the entry's index in `MENU`; the status item asks for the title, status, download and services items by entry, so reordering `MENU` cannot point an update at the wrong item.

### Settings selectors as a table
The 15 selector functions become an `Action` enum and one `match` from action to selector; `FormTarget::wire` takes an `Action`. `tag()` returns `None` for a sender that is not a control, and the target sends no event for it instead of a tag of `-1`.

### Config: parsed once, defaults by `Default`
Each section derives its missing keys from its own `Default` impl (`#[serde(default)]` on the struct), which drops the per-field default functions. `parse` deserializes `Config` straight from the text, keeping TOML's line and column in errors; only when that fails is the text parsed as a table to explain removed sections and keys, so a valid file is parsed once. The retired `debug_save_wav` leaves `AudioCfg` and the app's warning goes with it; the user's own file still sets it, so `AudioCfg` keeps a private ignored slot under that name and old files load unchanged. The settings window keeps listing it as retired.

### The handshake tries every address
`handshake` connects to the resolved addresses in order and keeps the first that accepts; the error is the last address's.

## Test levels

| Scenario | Level |
|---|---|
| The app and the scripts start the speech recognition service with the same arguments | adapter integration (`localservices_command_lines_it.rs`) |
| A changed argument in only one place is caught | adapter integration (`localservices_command_lines_it.rs`, a doctored `justfile` text) |
| Of two keyword spotting models the newest is used | unit (`catalog` tests) |

The refactors keep their behavior under the existing tests; new unit tests cover `MenuEntry::place`, the selector table, `tag` of a non-control, the config defaults without default functions, the retired key loading, and the handshake to a second address.
