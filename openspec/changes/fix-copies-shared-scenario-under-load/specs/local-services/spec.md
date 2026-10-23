# Spec Delta

## MODIFIED Requirements

### Requirement: The speech recognition service runs as several copies found from its address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, on the ports a client finds from the service's address; a request SHALL go to a copy that is free, SHALL wait its turn while every copy is busy, first come first served, and no copy SHALL get two requests at once. A request SHALL keep its place in the queue for as long as it waits for a copy, also when its wait runs out and it tries again. The sharing of requests that arrive at once SHALL hold while the machine is busy with another run: a check of it waits for every transcript as long as the queued transcriptions take on a loaded machine, and when a request still gets no transcript, the failure SHALL name that request, the copies it waited on and what the machine was busy with.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

#### Scenario: Requests that arrive at once are shared by the copies while the machine is busy
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies, and another run keeps the machine's processor busy
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript within the time the queued transcriptions take on the busy machine, and each of the three copies transcribed some of them

#### Scenario: A request without a transcript names the machine's load
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once and one of them gets no transcript in time
- **THEN** the failure names that request, the copies it waited on and the busiest processes on the machine

#### Scenario: Requests wait their turn when every copy is busy
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech
- **WHEN** two more requests arrive one after another
- **THEN** each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

#### Scenario: A request whose wait ran out keeps its place
- **GIVEN** the only copy of the speech recognition service is busy for longer than a request waits for a copy
- **WHEN** two requests arrive one after another, and the copy frees just after the first one's wait ran out
- **THEN** the first request is transcribed on the copy before the second one
