## MODIFIED Requirements

### Requirement: The speech recognition service runs as several copies found from its address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, on the ports a client finds from the service's address; a request SHALL go to a copy that is free, SHALL wait its turn while every copy is busy, first come first served, and no copy SHALL get two requests at once. A request SHALL keep its place in the queue for as long as it waits for a copy, also when its wait runs out and it tries again.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

#### Scenario: Requests wait their turn when every copy is busy
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech
- **WHEN** two more requests arrive one after another
- **THEN** each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

#### Scenario: A request whose wait ran out keeps its place
- **GIVEN** the only copy of the speech recognition service is busy for longer than a request waits for a copy
- **WHEN** two requests arrive one after another, and the copy frees just after the first one's wait ran out
- **THEN** the first request is transcribed on the copy before the second one
