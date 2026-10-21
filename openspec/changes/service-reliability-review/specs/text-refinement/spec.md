## MODIFIED Requirements

### Requirement: A dropped connection to the text refinement service is retried once
The system SHALL send the refinement request once more, to another copy of the text refinement service when one is up, when the copy it took refuses or drops the connection, or sends no answer within the request time limit, and SHALL log the failed attempt. When no copy accepts connections, the system SHALL first wait up to 30 seconds for one to come back. Only when the second attempt fails too, or no copy comes back, does refinement fall back to the raw transcript.

#### Scenario: The text refinement service drops one connection
- **GIVEN** the text refinement service drops the first connection and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the refined text is pasted and the dropped attempt is logged

#### Scenario: The text refinement service keeps dropping the connection
- **GIVEN** the text refinement service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the request is sent twice in total and the raw transcript is pasted

#### Scenario: A copy of the text refinement service is killed during a refinement
- **GIVEN** the app started the text refinement service as two copies
- **WHEN** the copy refining a transcript is killed before it answers
- **THEN** the other copy refines the transcript, and the text is not reported as pasted without refinement

#### Scenario: Every copy is down while the app restarts them
- **GIVEN** no copy of the text refinement service accepts connections, and one comes back 3 seconds later
- **WHEN** the user finishes a recording
- **THEN** the refined text is pasted once the copy answers, and the wait is logged

#### Scenario: A copy that sends no answer is replaced by another
- **GIVEN** the text refinement service runs as two copies, and one sends no answer
- **WHEN** a refinement request reaches that copy
- **THEN** after the request time limit the other copy refines the transcript
