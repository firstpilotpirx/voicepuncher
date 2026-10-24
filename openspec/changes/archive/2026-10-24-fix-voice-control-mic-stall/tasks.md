## 1. Microphone keeper

- [x] 1.1 Red: unit tests in `kept_mic.rs` — a stalled microphone is opened again and its audio is sent; an error is opened again; a failing reopen is retried until it succeeds; the silent time is sent as silence
- [x] 1.2 Green: `KeptMic`, `OpenMic`, `KeeperTimings` with the stall rule, retries, logging once and silence padding
- [x] 1.3 `WebSocketCommandSpotter::start_listening` opens the microphone through `KeptMic` and starts `AudioStream` at 16 kHz on the keeper's channel; `Listening` keeps the keeper
- [x] 1.4 `just test`; commit

## 2. Service logs written directly

- [x] 2.1 Red: the integration test in `app/tests/it/localservices_process.rs` checks that a started service's output is its log file, not a pipe
- [x] 2.2 Green: `RotatingFileWriter::into_file`; `ProcessLauncher::spawn` hands the child the log file as stdout and stderr; drop the copy threads and their unit tests
- [x] 2.3 `just test`; commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.38.1
- [x] 3.2 `just test`; commit
