# The app runs as its own binary with a home folder of its own and services that answer
# nowhere: with voice control on, its first service check comes before any permission check,
# so it stops there with exit code 2 after logging its header and the error. A terminal is
# the pty `script` gives it. The rotation scenarios drive the log file writer with a small
# limit instead of 10 MB. "A panic is logged" is the panic hook's unit test: a panic cannot
# be caused in the binary from outside.
@app_logging @component
Feature: app-logging

  Rule: The log is written to a file in hotkey mode

    Scenario: A session's lines are in the log file
      Given the logs folder does not exist yet
      When the app starts in hotkey mode without a terminal
      Then the logs folder exists and the log file holds the session's lines in the terminal line format

    Scenario: A later session appends to the same file
      Given the log file holds the lines of an earlier session
      When the app starts in hotkey mode without a terminal
      Then the earlier lines are still in the log file and the new session's lines follow them

  Rule: A terminal gets the log too

    Scenario: Started from a terminal
      Given the logs folder does not exist yet
      When the app starts in hotkey mode on a terminal
      Then the session's lines appear on the terminal and in the log file

    Scenario: Started without a terminal
      Given the logs folder does not exist yet
      When the app starts in hotkey mode without a terminal
      Then the session's lines are in the log file and nothing is written to the error output

  Rule: File mode logs to the error output only

    Scenario: Transcribing a file leaves no log file
      Given the logs folder does not exist yet
      When the app transcribes an audio file in file mode
      Then its log lines are on the error output and the logs folder still does not exist

  Rule: The log rotates by size

    Scenario: The limit is reached
      Given a log file just below its size limit
      When a line is written that would take it past the limit
      Then the former content is in the first backup and the new log file holds only that whole line

    Scenario: Backups shift and the oldest goes
      Given a log file with five backups
      When the log file rotates again
      Then each backup moved one number up, the former fifth backup is gone and six files remain

    Scenario: A session continues on an existing file
      Given a log file filled past half its limit by an earlier session
      When a new session writes as much again
      Then the log file rotated when it passed the limit, counting what was there before the session

  Rule: Every launch starts with a header

    Scenario: The header line
      Given the logs folder does not exist yet
      When the app starts in hotkey mode without a terminal
      Then the first line of the session names the version, the process id, the config path, the macOS version and the architecture
