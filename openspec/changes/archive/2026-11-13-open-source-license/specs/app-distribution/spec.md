## MODIFIED Requirements

### Requirement: Licenses ship with the bundle
The bundle SHALL carry the license text of every bundled engine and library it links in its resources, and the README SHALL name each of them with its license. The bundle SHALL also carry the application's own license texts, the repository's `LICENSE-MIT` and `LICENSE-APACHE`, unchanged in its resources.

#### Scenario: The bundle holds the licenses
- **GIVEN** the bundle the bundle build made
- **WHEN** its licenses folder is listed
- **THEN** it holds the licenses of whisper.cpp, ggml, the OpenMP runtime, sherpa-onnx and ONNX Runtime
- **AND** it holds no license of llama.cpp

#### Scenario: The bundle holds the application's own license
- **GIVEN** the bundle the bundle build made
- **WHEN** its resources are listed
- **THEN** they hold "LICENSE-MIT" and "LICENSE-APACHE" equal to the repository's files
