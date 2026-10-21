# app-logging Specification

## Purpose
The application's own log file: what the app writes about a session in hotkey mode, where it lives, how it rotates, and how it reaches a terminal when there is one, so that a session's log can be read after a launch without a terminal.

## Requirements

### Requirement: The log is written to a file in hotkey mode
In hotkey mode the application SHALL write every log line of the session to its log file in the logs folder of its home folder, in the same line format it uses on a terminal, and SHALL create the logs folder when it is missing.

#### Scenario: A session's lines are in the log file
- **GIVEN** the logs folder does not exist yet
- **WHEN** the application starts in hotkey mode and writes log lines
- **THEN** the logs folder exists and the log file holds those lines in the terminal line format

#### Scenario: A later session appends to the same file
- **GIVEN** the log file holds the lines of an earlier session
- **WHEN** the application starts in hotkey mode again
- **THEN** the earlier lines are still in the file and the new session's lines follow them

### Requirement: A terminal gets the log too
When the application's error output is a terminal, it SHALL write every log line to that terminal as well as to the log file; when the error output is not a terminal, it SHALL write no log line to the error output.

#### Scenario: Started from a terminal
- **GIVEN** the application's error output is a terminal
- **WHEN** the application writes a log line in hotkey mode
- **THEN** the line appears on the terminal and in the log file

#### Scenario: Started without a terminal
- **GIVEN** the application's error output is not a terminal
- **WHEN** the application writes a log line in hotkey mode
- **THEN** the line is in the log file and nothing is written to the error output

### Requirement: File mode logs to the error output only
In file mode the application SHALL write its log lines to the error output only and SHALL neither create nor write the log file or the logs folder.

#### Scenario: Transcribing a file leaves no log file
- **GIVEN** the logs folder does not exist
- **WHEN** the application transcribes an audio file in file mode
- **THEN** its log lines are on the error output and the logs folder still does not exist

### Requirement: The log rotates by size
When writing a line would take the log file past 10 MB, the application SHALL first move the current file to the first numbered backup, shift the earlier backups one number up keeping at most five, delete the oldest, and continue in a new file, so that no line is split across files and no line is lost.

#### Scenario: The limit is reached
- **GIVEN** the log file is just below 10 MB
- **WHEN** the application writes a line that would take it past 10 MB
- **THEN** the former content is in the first backup, the new file holds only that line, and the line is whole

#### Scenario: Backups shift and the oldest goes
- **GIVEN** five backups exist
- **WHEN** the log file rotates again
- **THEN** each backup moved one number up, the former fifth backup is gone, and six files at most remain

#### Scenario: A session continues on an existing file
- **GIVEN** the log file already holds 6 MB from earlier sessions
- **WHEN** the application writes 5 MB more in one session
- **THEN** the file rotates when it passes 10 MB, counting what was there before the session

### Requirement: Every launch starts with a header
The first log line of every session SHALL name the application's version, its process id, the config file path, the macOS version and the CPU architecture.

#### Scenario: The header line
- **WHEN** the application starts in hotkey mode
- **THEN** the first log line of the session names the version, the process id, the config path, the macOS version and the architecture

### Requirement: A panic is logged
When any thread of the application panics, the application SHALL write a log line with the panic message, the source location and the thread name before the process ends.

#### Scenario: A worker thread panics
- **GIVEN** the application is running in hotkey mode
- **WHEN** one of its threads panics
- **THEN** the log holds a line with the panic message, the file and line, and the thread's name
