# Spec Delta

## ADDED Requirements

### Requirement: The fast streaming bench reproduces the service's timing
In its fast pace the streaming bench SHALL replay the service's clock: each recognizer's decode becomes visible to the session at the stream time it would have ended on this machine when fed in real time (its measured decode time, queued behind the decode before it), so a command's reported delay differs from the real-time pace's by at most 0.1 s. The real-time pace SHALL keep the delay measured on the wall clock. The fast pace SHALL end its report by naming what the replay does not reproduce.

#### Scenario: A lone restart gives the same delay in both paces
- **GIVEN** the audio of a voice start, 20 s of dictation and a lone "restart" cut at its last sound, followed by silence
- **WHEN** the bench runs it in the fast pace and in the real-time pace
- **THEN** both paces report the "restart" command, and the delay from the end of its words to its report differs between the paces by at most 0.1 s

#### Scenario: The user's recordings give the same delays in both paces
- **GIVEN** a sample of the user's own recordings of the commands exists on the machine
- **WHEN** the bench runs the sample in the fast pace and in the real-time pace
- **THEN** every command the real-time pace reports is reported by the fast pace, and the delay of each differs between the paces by at most 0.1 s

#### Scenario: A decode is seen when it would have ended
- **GIVEN** a recognizer whose decode of a frame takes 0.3 s on this machine
- **WHEN** the fast pace feeds a command whose words end at 1.0 s of the stream
- **THEN** the session sees the recognizer's text of those words no earlier than 1.3 s of the stream, and the delay reported for the command counts from the end of its words to that stream time

#### Scenario: The fast report names what it does not replay
- **WHEN** the bench runs in the fast pace
- **THEN** its report ends with one line naming what the replay does not reproduce: a load other than its own, the service's poll and wake, the app's audio path and its syncs, and the granularity of a frame
