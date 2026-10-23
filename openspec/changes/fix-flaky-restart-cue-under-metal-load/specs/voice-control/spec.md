## MODIFIED Requirements

### Requirement: A voice start gives its start cue within 0.7 s
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play the sound that start plays (the restart sound for "restart") as soon as the command is confirmed, within 0.7 s of the end of the command's words in at least 9 of 10 starts, with the services otherwise idle; the overlay's recording sign and the recording follow once the microphone opens. In the scenarios below "the start cue" means that sound. This holds for "start" alone, opening or ending an utterance, or doubled, with "please" after it, and every form of "restart", also when the first hearing of a quiet "restart" takes it for another word. A check of this requirement SHALL time the start cue only on an idle machine: it waits for the builds, tests and model stages of every other check on the machine to end and keeps new ones out while it times, so the bound is measured under the condition it states. A start cue over its bound SHALL be reported with what ran beside it: the busiest processes on the machine and every check that held a lock at that moment. A check SHALL time the start cue only against warm copies of the speech recognition service for commands: each copy has transcribed speech since it started, so no confirmation of a timed start waits for a copy's first decode. A start cue that never came because the confirmation of the command was cut short SHALL be reported with the copies whose confirmation was cut short, how long each decoded, and what ran beside them, as a late cue is.

#### Scenario: Start start followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start start", while the sentence is still being said

#### Scenario: Start followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start", while the sentence is still being said

#### Scenario: Start said on its own gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and stays silent
- **THEN** the start cue comes within 0.7 s of the end of "start"

#### Scenario: Restart restart followed by the new dictation at once gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart restart" and another sentence without a pause
- **THEN** the start cue of the new dictation comes within 0.7 s of the end of "restart restart"

#### Scenario: Restart said on its own gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has dictated for about 20 s
- **WHEN** the user says "restart" and stays silent
- **THEN** the start cue of the new dictation comes within 0.7 s of the end of "restart"

#### Scenario: The user's recorded starts give their cue at once
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** listening is on
- **WHEN** each of the user's recorded starts is played, on its own and followed at once by a sentence
- **THEN** at least 9 of 10 of them give the start cue within 0.7 s of the end of the command's words

#### Scenario: A faster start keeps the dictation where the command's words end
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause, and later ends the dictation
- **THEN** the whole sentence, its first words included, is pasted into the focused app without "start"

#### Scenario: A faster start adds no false start
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says a sentence with "start" inside it, ordinary speech, a short Russian phrase or "start stop"
- **THEN** no dictation starts

#### Scenario: Start please followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start please" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start please", while the sentence is still being said

#### Scenario: Start please said on its own gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start please" and stays silent
- **THEN** the start cue comes within 0.7 s of the end of "start please"

#### Scenario: A brief dip inside the command words starts nothing early
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and "stop" with a gap between them shorter than a pause
- **THEN** no dictation starts

#### Scenario: The start cue is timed once the other checks' builds have ended
- **GIVEN** another check on the machine is building or testing while this check reaches its cue-timing scenarios
- **WHEN** the cue-timing scenarios are due
- **THEN** they wait for that build or test to end, no new build or test of another check starts beside them, and they time the start cue only then

#### Scenario: A late start cue names what ran beside it
- **GIVEN** the start cue of a voice start came later than its bound
- **WHEN** the failure is reported
- **THEN** the report names the busiest processes on the machine and every check that held a lock at that moment

#### Scenario: The start cue is timed against warm copies
- **GIVEN** the speech recognition service for commands was started for a check right before its cue-timing scenarios
- **WHEN** the first cue-timing scenario confirms its command
- **THEN** every copy of that service has already transcribed speech, and the confirmation does not wait for a copy's first decode

#### Scenario: A missing start cue names the confirmations cut short
- **GIVEN** the start cue of a voice start never came because the confirmation of its command was cut short on every copy
- **WHEN** the failure is reported
- **THEN** the report names each copy whose confirmation was cut short and how long it decoded, beside the busiest processes on the machine and every check that held a lock at that moment
